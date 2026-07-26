#![allow(unused_imports)]
use covopt_macro::covopt_param;
use std::io::Write;
use script_go::assembler::parse_asm;
use script_go::sgl::host_handlers::{HostContext, SysCallCommand};
use script_go::sgl::instruction::{Instruction, OpCode};
use script_go::sgl::vm::ScriptVm;

#[test]
fn test_vm_memory_payload_helpers() {
    let mut vm = ScriptVm::new();

    // Test write_string and read_string
    let sample_text = "Hello SGL Memory Helpers!";
    let bytes_written = vm.write_string(covopt_param!("M_16_40", 64), sample_text, true).unwrap();
    assert_eq!(bytes_written, sample_text.len() + 1);

    let read_text = vm.read_string(covopt_param!("M_19_35", 64), None).unwrap();
    assert_eq!(read_text, sample_text);

    // Test write_bytes and read_bytes
    let raw_payload: [u8; 5] = [covopt_param!("M_23_32", 10), covopt_param!("M_23_36", 20), covopt_param!("M_23_40", 30), covopt_param!("M_23_44", 40), covopt_param!("M_23_48", 50)];
    vm.write_bytes(covopt_param!("M_24_19", 128), &raw_payload).unwrap();
    let read_payload = vm.read_bytes(covopt_param!("M_25_37", 128), covopt_param!("M_25_42", 5), false).unwrap();
    assert_eq!(read_payload, raw_payload);
}

#[test]
fn test_networking_handlers() {
    let mut vm = ScriptVm::new();
    let mut host_context = HostContext::new();

    // Add a custom mock route
    host_context.http_mock_routes.insert(
        "https://api.test.com/data".to_string(),
        "{\"result\":\"success\"}".to_string(),
    );
    vm.register_host_context(host_context);

    // 1. Test NetworkStatus (Cmd 6)
    let code_net_status = [
        Instruction::new(OpCode::HardwareCall as u8, 1, covopt_param!("M_43_56", 6), covopt_param!("M_43_59", 6)),
        Instruction::new(OpCode::Halt as u8, 0, 0, 0),
    ];
    vm.run(&code_net_status).unwrap();
    assert_eq!(vm.registers[1], 1); // Online by default

    // Toggle network status
    vm.get_host_context_mut().unwrap().network_online = false;
    vm.pc = 0;
    vm.run(&code_net_status).unwrap();
    assert_eq!(vm.registers[1], 0);

    // Re-enable network
    vm.get_host_context_mut().unwrap().network_online = true;

    // 2. Test HttpGet (Cmd 1)
    let url = "https://api.test.com/data";
    vm.write_string(covopt_param!("M_60_20", 100), url, true).unwrap();
    let code_http_get = [
        Instruction::new(OpCode::LoadImm as u8, 2, covopt_param!("M_62_51", 100), 0), // R[2] = 100
        Instruction::new(OpCode::HardwareCall as u8, covopt_param!("M_63_53", 3), 1, 2), // R[3] = HttpGet(R[2])
        Instruction::new(OpCode::Halt as u8, 0, 0, 0),
    ];
    vm.pc = 0;
    vm.run(&code_http_get).unwrap();
    let response_address = vm.registers[covopt_param!("M_68_40", 3)] as usize;
    assert!(response_address > 0);
    let response_text = vm.read_string(response_address, None).unwrap();
    assert_eq!(response_text, "{\"result\":\"success\"}");

    // 3. Test HttpPost (Cmd 2)
    let post_url = "https://api.test.com/submit";
    let post_body = "payload_content";
    vm.write_string(covopt_param!("M_76_20", 200), post_url, true).unwrap();
    vm.write_string(covopt_param!("M_77_20", 250), post_body, true).unwrap();
    vm.registers[covopt_param!("M_78_17", 10)] = covopt_param!("M_78_23", 200); // URL pointer
    vm.registers[covopt_param!("M_79_17", 11)] = covopt_param!("M_79_23", 250); // Body pointer
    let code_http_post = [
        Instruction::new(OpCode::HardwareCall as u8, covopt_param!("M_81_53", 4), 2, covopt_param!("M_81_59", 10)), // R[4] = HttpPost(url: R[10], body: R[11])
        Instruction::new(OpCode::Halt as u8, 0, 0, 0),
    ];
    vm.pc = 0;
    vm.run(&code_http_post).unwrap();
    let post_resp_addr = vm.registers[covopt_param!("M_86_38", 4)] as usize;
    assert!(post_resp_addr > 0);
    let post_resp_text = vm.read_string(post_resp_addr, None).unwrap();
    assert!(post_resp_text.contains("posted_to"));

    // 4. Test Socket Operations: SocketConnect, SocketSend, SocketRecv
    let socket_addr = "127.0.0.1:9000";
    vm.write_string(covopt_param!("M_93_20", 300), socket_addr, true).unwrap();
    vm.registers[covopt_param!("M_94_17", 12)] = covopt_param!("M_94_23", 300);
    let code_socket_connect = [
        Instruction::new(OpCode::HardwareCall as u8, covopt_param!("M_96_53", 5), covopt_param!("M_96_56", 3), covopt_param!("M_96_59", 12)),
        Instruction::new(OpCode::Halt as u8, 0, 0, 0),
    ];
    vm.pc = 0;
    vm.run(&code_socket_connect).unwrap();
    let socket_id = vm.registers[covopt_param!("M_101_33", 5)];
    assert_eq!(socket_id, 1);

    // SocketSend
    let send_data = "Socket Buffer Test";
    vm.write_string(covopt_param!("M_106_20", 350), send_data, false).unwrap();
    vm.registers[covopt_param!("M_107_17", 13)] = socket_id;
    vm.registers[covopt_param!("M_108_17", 14)] = covopt_param!("M_108_23", 350); // buffer addr
    vm.registers[covopt_param!("M_109_17", 15)] = send_data.len() as u32; // length
    let code_socket_send = [
        Instruction::new(OpCode::HardwareCall as u8, covopt_param!("M_111_53", 6), covopt_param!("M_111_56", 4), covopt_param!("M_111_59", 13)), // Cmd 4 = SocketSend
        Instruction::new(OpCode::Halt as u8, 0, 0, 0),
    ];
    vm.pc = 0;
    vm.run(&code_socket_send).unwrap();
    assert_eq!(vm.registers[6], send_data.len() as u32);
}

#[test]
fn test_file_and_system_io_handlers() {
    let mut vm = ScriptVm::new();
    let host_context = HostContext::new();
    vm.register_host_context(host_context);

    // 1. Test GetTimestamp (Cmd 3)
    vm.registers[1] = SysCallCommand::GetTimestamp as u32;
    let code_timestamp = [
        Instruction::new(OpCode::SysCall as u8, 2, 1, 0),
        Instruction::new(OpCode::Halt as u8, 0, 0, 0),
    ];
    vm.run(&code_timestamp).unwrap();
    let timestamp = vm.registers[2];
    assert!(timestamp > 0);

    // 2. Test GetEnv (Cmd 4)
    vm.write_string(covopt_param!("M_136_20", 100), "ENV", true).unwrap();
    vm.registers[covopt_param!("M_137_17", 3)] = SysCallCommand::GetEnv as u32;
    let code_getenv = [
        Instruction::new(OpCode::LoadImm as u8, covopt_param!("M_139_48", 4), covopt_param!("M_139_51", 100), 0),
        Instruction::new(OpCode::SysCall as u8, covopt_param!("M_140_48", 5), covopt_param!("M_140_51", 3), covopt_param!("M_140_54", 4)),
        Instruction::new(OpCode::Halt as u8, 0, 0, 0),
    ];
    vm.pc = 0;
    vm.run(&code_getenv).unwrap();
    let env_val_addr = vm.registers[covopt_param!("M_145_36", 5)] as usize;
    assert!(env_val_addr > 0);
    let env_val = vm.read_string(env_val_addr, None).unwrap();
    assert_eq!(env_val, "production");

    // 3. Test FileWrite (Cmd 2) and FileRead (Cmd 1)
    let file_path = "output.log";
    let file_content = "Log entry data";
    vm.write_string(covopt_param!("M_153_20", 200), file_path, true).unwrap();
    vm.write_string(covopt_param!("M_154_20", 250), file_content, false).unwrap();

    vm.registers[covopt_param!("M_156_17", 10)] = SysCallCommand::FileWrite as u32;
    vm.registers[covopt_param!("M_157_17", 11)] = covopt_param!("M_157_23", 200); // path addr
    vm.registers[covopt_param!("M_158_17", 12)] = covopt_param!("M_158_23", 250); // data addr
    vm.registers[covopt_param!("M_159_17", 13)] = file_content.len() as u32; // data len

    let code_file_write = [
        Instruction::new(OpCode::SysCall as u8, covopt_param!("M_162_48", 6), covopt_param!("M_162_51", 10), covopt_param!("M_162_55", 11)),
        Instruction::new(OpCode::Halt as u8, 0, 0, 0),
    ];
    vm.pc = 0;
    vm.run(&code_file_write).unwrap();
    assert_eq!(vm.registers[6], file_content.len() as u32);

    // FileRead
    vm.registers[covopt_param!("M_170_17", 20)] = SysCallCommand::FileRead as u32;
    let code_file_read = [
        Instruction::new(OpCode::LoadImm as u8, covopt_param!("M_172_48", 21), covopt_param!("M_172_52", 200), 0),
        Instruction::new(OpCode::SysCall as u8, covopt_param!("M_173_48", 7), covopt_param!("M_173_51", 20), covopt_param!("M_173_55", 21)),
        Instruction::new(OpCode::Halt as u8, 0, 0, 0),
    ];
    vm.pc = 0;
    vm.run(&code_file_read).unwrap();
    let read_file_addr = vm.registers[covopt_param!("M_178_38", 7)] as usize;
    assert!(read_file_addr > 0);
    let read_file_content = vm.read_string(read_file_addr, None).unwrap();
    assert_eq!(read_file_content, file_content);
}

#[test]
fn test_string_manipulation_handlers() {
    let mut vm = ScriptVm::new();
    let host_context = HostContext::new();
    vm.register_host_context(host_context);

    let str1 = "Hello, ";
    let str2 = "SGL Runtime!";
    vm.write_string(covopt_param!("M_192_20", 100), str1, true).unwrap();
    vm.write_string(covopt_param!("M_193_20", 150), str2, true).unwrap();

    // 1. StringConcat (Cmd 5)
    vm.registers[1] = SysCallCommand::StringConcat as u32;
    vm.registers[2] = covopt_param!("M_197_22", 100);
    vm.registers[covopt_param!("M_198_17", 3)] = covopt_param!("M_198_22", 150);
    let code_concat = [
        Instruction::new(OpCode::SysCall as u8, covopt_param!("M_200_48", 10), 1, 2),
        Instruction::new(OpCode::Halt as u8, 0, 0, 0),
    ];
    vm.run(&code_concat).unwrap();
    let concat_addr = vm.registers[covopt_param!("M_204_35", 10)] as usize;
    let concat_str = vm.read_string(concat_addr, None).unwrap();
    assert_eq!(concat_str, "Hello, SGL Runtime!");

    // 2. StringLength (Cmd 6)
    vm.registers[covopt_param!("M_209_17", 4)] = SysCallCommand::StringLength as u32;
    let code_length = [
        Instruction::new(OpCode::SysCall as u8, covopt_param!("M_211_48", 11), covopt_param!("M_211_52", 4), covopt_param!("M_211_55", 10)), // length of concatenated string
        Instruction::new(OpCode::Halt as u8, 0, 0, 0),
    ];
    vm.pc = 0;
    vm.run(&code_length).unwrap();
    assert_eq!(vm.registers[11], 19);

    // 3. StringSlice (Cmd 7)
    let full_string = "ScriptGo Engine";
    vm.write_string(covopt_param!("M_220_20", 200), full_string, true).unwrap();
    vm.registers[covopt_param!("M_221_17", 5)] = SysCallCommand::StringSlice as u32;
    vm.registers[covopt_param!("M_222_17", 6)] = covopt_param!("M_222_22", 200); // string ptr
    vm.registers[covopt_param!("M_223_17", 7)] = 0;   // start
    vm.registers[covopt_param!("M_224_17", 8)] = covopt_param!("M_224_22", 8);   // end
    let code_slice = [
        Instruction::new(OpCode::SysCall as u8, covopt_param!("M_226_48", 12), covopt_param!("M_226_52", 5), covopt_param!("M_226_55", 6)),
        Instruction::new(OpCode::Halt as u8, 0, 0, 0),
    ];
    vm.pc = 0;
    vm.run(&code_slice).unwrap();
    let slice_addr = vm.registers[covopt_param!("M_231_34", 12)] as usize;
    let slice_str = vm.read_string(slice_addr, None).unwrap();
    assert_eq!(slice_str, "ScriptGo");

    // 4. StringToUpper (Cmd 8) and StringToLower (Cmd 9)
    vm.registers[covopt_param!("M_236_17", 15)] = SysCallCommand::StringToUpper as u32;
    vm.registers[covopt_param!("M_237_17", 16)] = covopt_param!("M_237_23", 200);
    let code_upper = [
        Instruction::new(OpCode::SysCall as u8, covopt_param!("M_239_48", 13), covopt_param!("M_239_52", 15), covopt_param!("M_239_56", 16)),
        Instruction::new(OpCode::Halt as u8, 0, 0, 0),
    ];
    vm.pc = 0;
    vm.run(&code_upper).unwrap();
    let upper_addr = vm.registers[covopt_param!("M_244_34", 13)] as usize;
    let upper_str = vm.read_string(upper_addr, None).unwrap();
    assert_eq!(upper_str, "SCRIPTGO ENGINE");

    vm.registers[covopt_param!("M_248_17", 17)] = SysCallCommand::StringToLower as u32;
    vm.registers[covopt_param!("M_249_17", 18)] = covopt_param!("M_249_23", 200);
    let code_lower = [
        Instruction::new(OpCode::SysCall as u8, covopt_param!("M_251_48", 14), covopt_param!("M_251_52", 17), covopt_param!("M_251_56", 18)),
        Instruction::new(OpCode::Halt as u8, 0, 0, 0),
    ];
    vm.pc = 0;
    vm.run(&code_lower).unwrap();
    let lower_addr = vm.registers[covopt_param!("M_256_34", 14)] as usize;
    let lower_str = vm.read_string(lower_addr, None).unwrap();
    assert_eq!(lower_str, "scriptgo engine");
}

#[test]
fn test_sgo_sample_scripts_execution() {
    let syscall_asm = std::fs::read_to_string("tests/sample_syscalls.sgo").unwrap();
    let syscall_code = parse_asm(&syscall_asm).unwrap();

    let mut vm = ScriptVm::new();
    let host_context = HostContext::new();
    vm.register_host_context(host_context);

    // Setup input strings in VM memory
    vm.write_string(0, "ENV", true).unwrap();
    vm.write_string(covopt_param!("M_272_20", 30), "Part1", true).unwrap();
    vm.write_string(covopt_param!("M_273_20", 40), "Part2", true).unwrap();

    vm.run(&syscall_code).unwrap();

    // Verify timestamp was generated
    assert!(vm.registers[10] > 0);

    let hardware_asm = std::fs::read_to_string("tests/sample_hardware_calls.sgo").unwrap();
    let hardware_code = parse_asm(&hardware_asm).unwrap();

    let mut vm_hw = ScriptVm::new();
    let host_hw_context = HostContext::new();
    vm_hw.register_host_context(host_hw_context);

    vm_hw.run(&hardware_code).unwrap();
    // Verify NetworkStatus returned online (1)
    assert_eq!(vm_hw.registers[20], 1);
}
