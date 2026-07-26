#![allow(unused_imports)]
use covopt_macro::covopt_param;
use std::io::Write;
use script_go::sgl::host_handlers::HostContext;
use script_go::sgl::instruction::{Instruction, OpCode};
use script_go::sgl::vm::ScriptVm;
use script_go::{sgl_combine_handlers, sgl_package};
use std::sync::atomic::{AtomicU32, Ordering};

// Test 1 counters & packages
static T1_A_COUNT: AtomicU32 = AtomicU32::new(0);
static T1_B_COUNT: AtomicU32 = AtomicU32::new(0);

#[sgl_package(name = "t1_pkg_a", kind = "hardware")]
pub mod t1_pkg_a {
    use super::T1_A_COUNT;
    use script_go::sgl::vm::ScriptVm;
    use std::sync::atomic::Ordering;

    #[sgl_cmd(id = 1)]
    pub fn return_zero_cmd(_vm: &mut ScriptVm, _dummy: String) -> u32 {
        T1_A_COUNT.fetch_add(1, Ordering::SeqCst);
        0 // Returns 0 as valid value
    }
}

#[sgl_package(name = "t1_pkg_b", kind = "hardware")]
pub mod t1_pkg_b {
    use super::T1_B_COUNT;
    use script_go::sgl::vm::ScriptVm;
    use std::sync::atomic::Ordering;

    #[sgl_cmd(id = 1)]
    pub fn t1_pkg_b_cmd_one(_vm: &mut ScriptVm, _dummy: String) -> u32 {
        T1_B_COUNT.fetch_add(1, Ordering::SeqCst);
        999
    }
}

// Test 2 counters & packages
static T2_A_COUNT: AtomicU32 = AtomicU32::new(0);
static T2_B_COUNT: AtomicU32 = AtomicU32::new(0);

#[sgl_package(name = "t2_pkg_a", kind = "hardware")]
pub mod t2_pkg_a {
    use super::T2_A_COUNT;
    use script_go::sgl::vm::ScriptVm;
    use std::sync::atomic::Ordering;

    #[sgl_cmd(id = 3)]
    pub fn string_arg_cmd(_vm: &mut ScriptVm, input: String) -> u32 {
        T2_A_COUNT.fetch_add(1, Ordering::SeqCst);
        input.len() as u32
    }
}

#[sgl_package(name = "t2_pkg_b", kind = "hardware")]
pub mod t2_pkg_b {
    use super::T2_B_COUNT;
    use script_go::sgl::vm::ScriptVm;
    use std::sync::atomic::Ordering;

    #[sgl_cmd(id = 3)]
    pub fn t2_pkg_b_cmd_three(_vm: &mut ScriptVm, _val: u32) -> u32 {
        T2_B_COUNT.fetch_add(1, Ordering::SeqCst);
        777
    }
}

// Test 3 counter & package
static T3_A_COUNT: AtomicU32 = AtomicU32::new(0);

#[sgl_package(name = "t3_pkg_a", kind = "hardware")]
pub mod t3_pkg_a {
    use super::T3_A_COUNT;
    use script_go::sgl::vm::ScriptVm;
    use std::sync::atomic::Ordering;

    #[sgl_cmd(id = 1)]
    pub fn cmd_one(_vm: &mut ScriptVm, _dummy: String) -> u32 {
        T3_A_COUNT.fetch_add(1, Ordering::SeqCst);
        42
    }
}

/// Challenge 1:
/// Prove that sgl_combine_handlers! falls through to Pkg B when Pkg A returns 0
/// and initial_dest is 0.
#[test]
fn test_adversarial_combine_handlers_fallthrough_on_zero_return() {
    let mut vm = ScriptVm::new();

    let combined = sgl_combine_handlers!(t1_pkg_a::dispatch, t1_pkg_b::dispatch);
    vm.register_hardware_handler(combined);

    vm.write_string(100, "hello", true).unwrap();

    // R[1] = dest_reg (default 0)
    // R[2] = cmd_reg (holds command ID 1)
    // R[3] = arg_reg (points to 100)
    vm.registers[1] = 0; // initial_dest = 0
    vm.registers[2] = 1;
    vm.registers[3] = 100;

    let inst = Instruction::new(OpCode::HardwareCall as u8, 1, 2, 3);
    let code = [inst, Instruction::new(OpCode::Halt as u8, 0, 0, 0)];

    let res = vm.run(&code);
    assert!(res.is_ok());

    let count_a = T1_A_COUNT.load(Ordering::SeqCst);
    let count_b = T1_B_COUNT.load(Ordering::SeqCst);

    std::io::stdout().write_fmt(format_args!("T1_A_COUNT: {}", count_a)).unwrap();
std::io::stdout().write_all(b"\n").unwrap();
    std::io::stdout().write_fmt(format_args!("T1_B_COUNT: {}", count_b)).unwrap();
std::io::stdout().write_all(b"\n").unwrap();
    std::io::stdout().write_fmt(format_args!("vm.registers[1]: {}", vm.registers[1])).unwrap();
std::io::stdout().write_all(b"\n").unwrap();

    // DEMONSTRATION OF BUG:
    // Pkg A executed (count_a = 1) and returned 0.
    // Because initial_dest was 0 and vm.registers[1] is 0, (0 != 0) is FALSE.
    // sgl_combine_handlers! incorrectly fell through and ALSO invoked Pkg B (count_b = 1)!
    assert_eq!(count_a, 1, "Pkg A should have executed once");
    assert_eq!(
        count_b, 1,
        "VULNERABILITY CONFIRMED: sgl_combine_handlers! incorrectly fell through to Pkg B!"
    );
    assert_eq!(
        vm.registers[1], 999,
        "VULNERABILITY CONFIRMED: Return register overwritten by Pkg B!"
    );
}

/// Challenge 2:
/// Prove that early return in string extraction (on invalid UTF-8) when initial_dest is 0 falls through to Pkg B.
#[test]
fn test_adversarial_combine_handlers_fallthrough_on_invalid_string_param() {
    let mut vm = ScriptVm::new();
    let combined = sgl_combine_handlers!(t2_pkg_a::dispatch, t2_pkg_b::dispatch);
    vm.register_hardware_handler(combined);

    // Write invalid UTF-8 bytes at memory 100
    vm.write_bytes(100, &[255, 255, 255, 0x00]).unwrap();

    // R[1] = dest_reg (0)
    // R[2] = cmd_reg (3 - string_arg_cmd)
    // R[3] = arg_reg pointing to address 100 with invalid UTF-8
    vm.registers[1] = 0; // initial_dest = 0
    vm.registers[2] = 3;
    vm.registers[3] = 100;

    let inst = Instruction::new(OpCode::HardwareCall as u8, 1, 2, 3);
    let code = [inst, Instruction::new(OpCode::Halt as u8, 0, 0, 0)];

    let res = vm.run(&code);
    assert!(res.is_ok());

    let count_a = T2_A_COUNT.load(Ordering::SeqCst);
    let count_b = T2_B_COUNT.load(Ordering::SeqCst);

    std::io::stdout().write_fmt(format_args!("T2_A_COUNT: {}", count_a)).unwrap();
std::io::stdout().write_all(b"\n").unwrap();
    std::io::stdout().write_fmt(format_args!("T2_B_COUNT: {}", count_b)).unwrap();
std::io::stdout().write_all(b"\n").unwrap();

    // Pkg A early-returned on invalid string without calling func body, setting R[1] = 0.
    // Because initial_dest was 0, sgl_combine_handlers! fallthrough triggered Pkg B!
    assert_eq!(count_a, 0, "Pkg A function body was not called due to param error");
    assert_eq!(
        count_b, 1,
        "VULNERABILITY CONFIRMED: Parameter extraction failure fell through to Pkg B!"
    );
}

/// Challenge 3:
/// Multi-package dispatching `cmd_id` resolution fallback when `vm.registers[cmd_reg] == 0`.
#[test]
fn test_adversarial_cmd_id_resolution_fallback_to_register_index() {
    let mut vm = ScriptVm::new();
    t3_pkg_a::register(&mut vm);

    vm.write_string(100, "test", true).unwrap();

    // HardwareCall dest=R0, cmd_reg=R1, arg_reg=R3.
    // Notice R1 holds value 0! (uninitialized / 0)
    vm.registers[0] = 0;
    vm.registers[1] = 0; // R1 = 0!
    vm.registers[3] = 100;

    let inst = Instruction::new(OpCode::HardwareCall as u8, 0, 1, 3);
    let code = [inst, Instruction::new(OpCode::Halt as u8, 0, 0, 0)];

    let res = vm.run(&code);
    assert!(res.is_ok());

    let count = T3_A_COUNT.load(Ordering::SeqCst);
    std::io::stdout().write_fmt(format_args!("T3_A_COUNT: {}", count)).unwrap();
std::io::stdout().write_all(b"\n").unwrap();

    // Because vm.registers[1] was 0, dispatch fell back to direct_cmd = cmd_reg = 1!
    // So t3_pkg_a executed command 1 (cmd_one)!
    assert_eq!(
        count, 1,
        "VULNERABILITY CONFIRMED: dispatch fell back to cmd_reg index when register held 0!"
    );
}

/// Challenge 4:
/// Host Context Scratch Heap Memory Allocation Overflow.
#[test]
fn test_adversarial_host_context_heap_overflow() {
    let mut ctx = HostContext::new();

    // Allocate 600 bytes
    let addr1 = ctx.allocate_vm_memory(600);
    assert_eq!(addr1, 512);

    // Scratch heap pointer is now 512 + 600 = 1112.
    // Allocate another 10 bytes:
    // 1112 + 12 > 1000, so it resets scratch_heap_pointer to 512!
    let addr2 = ctx.allocate_vm_memory(10);
    assert_eq!(
        addr2, 512,
        "VULNERABILITY CONFIRMED: Heap allocation returned duplicate address 512!"
    );

    // Now test with VM memory bounds: VM memory is [u8; 1024].
    let mut vm = ScriptVm::new();
    let write_res = vm.write_bytes(512, &[0u8; 600]);
    assert!(
        write_res.is_err(),
        "VULNERABILITY CONFIRMED: 600-byte allocation exceeds VM 1024-byte boundary!"
    );
}
