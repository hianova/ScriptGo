#![allow(unused_imports)]
use covopt_macro::covopt_param;
use std::io::Write;
use script_go::sgl::host_handlers::HostContext;
use script_go::sgl::instruction::{Instruction, OpCode};
use script_go::sgl::vm::ScriptVm;
use script_go::{SglIoRegisterExt, SglNetRegisterExt};
use script_go::sgl::net::*;
use script_go::sgl::io::*;

#[test]
fn test_adversarial_invalid_urls() {
    let mut vm = ScriptVm::new();
    let mut ctx = HostContext::new();
    ctx.http_mock_routes.insert(
        "https://valid.route".to_string(),
        "OK".to_string(),
    );
    vm.register_host_context(ctx);
    vm.register_sgl_net();

    // 1. Invalid unmapped memory address 0xDEADBEEF for HttpGet
    vm.registers[1] = covopt_param!("M_24_22", 999);
    vm.registers[2] = 1; // HttpGet
    vm.registers[covopt_param!("M_26_17", 3)] = covopt_param!("M_26_22", 3735928559);
    let code1 = [
        Instruction::new(OpCode::HardwareCall as u8, 1, 2, covopt_param!("M_28_59", 3)),
        Instruction::new(OpCode::Halt as u8, 0, 0, 0),
    ];
    let res1 = vm.run(&code1);
    assert!(res1.is_ok());
    assert_eq!(vm.registers[1], 0);

    // 2. Memory address at boundary (1023) without null terminator
    vm.pc = 0;
    vm.memory[covopt_param!("M_37_14", 1023)] = b'A'; // non-null at end of memory
    vm.registers[1] = covopt_param!("M_38_22", 999);
    vm.registers[2] = 1;
    vm.registers[covopt_param!("M_40_17", 3)] = covopt_param!("M_40_22", 1023);
    let code2 = [
        Instruction::new(OpCode::HardwareCall as u8, 1, 2, covopt_param!("M_42_59", 3)),
        Instruction::new(OpCode::Halt as u8, 0, 0, 0),
    ];
    let res2 = vm.run(&code2);
    assert!(res2.is_ok());

    // 3. Empty string URL
    vm.pc = 0;
    vm.write_string(covopt_param!("M_50_20", 100), "", true).unwrap();
    vm.registers[1] = covopt_param!("M_51_22", 999);
    vm.registers[2] = 1;
    vm.registers[covopt_param!("M_53_17", 3)] = covopt_param!("M_53_22", 100);
    let code3 = [
        Instruction::new(OpCode::HardwareCall as u8, 1, 2, covopt_param!("M_55_59", 3)),
        Instruction::new(OpCode::Halt as u8, 0, 0, 0),
    ];
    let res3 = vm.run(&code3);
    assert!(res3.is_ok());
    let ptr3 = vm.registers[1] as usize;
    assert!(ptr3 >= 512);
    let resp3 = vm.read_string(ptr3, None).unwrap();
    assert!(resp3.contains("Mock Response for "));

    // 4. Invalid UTF-8 bytes in URL memory
    vm.pc = 0;
    vm.write_bytes(covopt_param!("M_67_19", 200), &[covopt_param!("M_67_26", 255), covopt_param!("M_67_31", 254), covopt_param!("M_67_36", 253), 0x00]).unwrap();
    vm.registers[1] = covopt_param!("M_68_22", 999);
    vm.registers[2] = 1;
    vm.registers[covopt_param!("M_70_17", 3)] = covopt_param!("M_70_22", 200);
    let code4 = [
        Instruction::new(OpCode::HardwareCall as u8, 1, 2, covopt_param!("M_72_59", 3)),
        Instruction::new(OpCode::Halt as u8, 0, 0, 0),
    ];
    let res4 = vm.run(&code4);
    assert!(res4.is_ok());

    // 5. Direct sgl_net::http_get call with empty and malformed string
    let get_res_empty = http_get(&mut vm, "".to_string());
    assert_eq!(get_res_empty, Err(400));

    let get_res_mock = http_get(&mut vm, "https://valid.route".to_string());
    assert_eq!(get_res_mock, Ok("OK".to_string()));

    let get_res_default = http_get(&mut vm, "https://unregistered.route".to_string());
    assert!(get_res_default.is_ok());
}

#[test]
fn test_adversarial_http_post() {
    let mut vm = ScriptVm::new();
    vm.register_sgl_net();

    // 1. HttpPost with unmapped URL and Body addresses
    vm.registers[1] = covopt_param!("M_95_22", 999);
    vm.registers[2] = 2; // HttpPost
    vm.registers[covopt_param!("M_97_17", 3)] = covopt_param!("M_97_22", 200); // R[3]=200 -> arg_reg (R[200]=unmapped_url, R[201]=unmapped_body)
    vm.registers[covopt_param!("M_98_17", 200)] = covopt_param!("M_98_24", 3735928559); // url_addr (unmapped)
    vm.registers[covopt_param!("M_99_17", 201)] = covopt_param!("M_99_24", 3405691582); // body_addr (unmapped)
    let code1 = [
        Instruction::new(OpCode::HardwareCall as u8, 1, 2, covopt_param!("M_101_59", 3)),
        Instruction::new(OpCode::Halt as u8, 0, 0, 0),
    ];
    let res1 = vm.run(&code1);
    assert!(res1.is_ok());
    let ptr1 = vm.registers[1] as usize;
    assert!(ptr1 >= 512);
    let resp1 = vm.read_string(ptr1, None).unwrap();
    assert!(resp1.contains("posted_to"));

    // 2. Direct http_post package function
    let post_res = http_post(&mut vm, "http://test".to_string(), "body_content".to_string());
    assert!(post_res.is_ok());
    let post_str = post_res.unwrap();
    assert!(post_str.contains("bytes_received\":12"));
}

#[test]
fn test_adversarial_invalid_file_paths() {
    let mut vm = ScriptVm::new();
    let mut ctx = HostContext::new();
    ctx.virtual_filesystem.insert("secret.txt".to_string(), b"topsecret".to_vec());
    vm.register_host_context(ctx);
    vm.register_sgl_io();

    // 1. FileRead with unmapped path pointer
    vm.registers[1] = covopt_param!("M_127_22", 999);
    vm.registers[2] = 1; // FileRead
    vm.registers[covopt_param!("M_129_17", 3)] = covopt_param!("M_129_22", 3735928559);
    let code1 = [
        Instruction::new(OpCode::SysCall as u8, 1, 2, covopt_param!("M_131_54", 3)),
        Instruction::new(OpCode::Halt as u8, 0, 0, 0),
    ];
    let res1 = vm.run(&code1);
    assert!(res1.is_ok());
    assert_eq!(vm.registers[1], 0);

    // 2. FileRead non-existent file
    vm.pc = 0;
    vm.write_string(covopt_param!("M_140_20", 100), "nonexistent.txt", true).unwrap();
    vm.registers[1] = covopt_param!("M_141_22", 999);
    vm.registers[2] = 1;
    vm.registers[covopt_param!("M_143_17", 3)] = covopt_param!("M_143_22", 100);
    let code2 = [
        Instruction::new(OpCode::SysCall as u8, 1, 2, covopt_param!("M_145_54", 3)),
        Instruction::new(OpCode::Halt as u8, 0, 0, 0),
    ];
    let res2 = vm.run(&code2);
    assert!(res2.is_ok());
    assert_eq!(vm.registers[1], 0);

    // 3. FileRead empty path string
    vm.pc = 0;
    vm.write_string(covopt_param!("M_154_20", 100), "", true).unwrap();
    vm.registers[1] = covopt_param!("M_155_22", 999);
    vm.registers[2] = 1;
    vm.registers[covopt_param!("M_157_17", 3)] = covopt_param!("M_157_22", 100);
    let code3 = [
        Instruction::new(OpCode::SysCall as u8, 1, 2, covopt_param!("M_159_54", 3)),
        Instruction::new(OpCode::Halt as u8, 0, 0, 0),
    ];
    let res3 = vm.run(&code3);
    assert!(res3.is_ok());
    assert_eq!(vm.registers[1], 0);

    // 4. Direct sgl_io::file_read call
    let read_empty = file_read(&mut vm, "".to_string());
    assert_eq!(read_empty, Err(400));

    let read_missing = file_read(&mut vm, "missing.txt".to_string());
    assert_eq!(read_missing, Err(404));

    let read_ok = file_read(&mut vm, "secret.txt".to_string());
    assert_eq!(read_ok, Ok("topsecret".to_string()));
}

#[test]
fn test_adversarial_file_write_out_of_bounds() {
    let mut vm = ScriptVm::new();
    let ctx = HostContext::new();
    vm.register_host_context(ctx);
    vm.register_sgl_io();

    // FileWrite with data_address + length crossing VM memory boundary (1024)
    vm.write_string(covopt_param!("M_185_20", 100), "overflow.txt", true).unwrap();
    vm.registers[1] = covopt_param!("M_186_22", 999); // dest
    vm.registers[2] = 2;   // FileWrite
    vm.registers[covopt_param!("M_188_17", 3)] = covopt_param!("M_188_22", 200); // arg_reg -> R[200]=100 (path), R[201]=1020 (data addr), R[202]=100 (len)
    vm.registers[covopt_param!("M_189_17", 200)] = covopt_param!("M_189_24", 100);
    vm.registers[covopt_param!("M_190_17", 201)] = covopt_param!("M_190_24", 1020);
    vm.registers[covopt_param!("M_191_17", 202)] = covopt_param!("M_191_24", 100); // 1020 + 100 > 1024 boundary

    let code = [
        Instruction::new(OpCode::SysCall as u8, 1, 2, covopt_param!("M_194_54", 3)),
        Instruction::new(OpCode::Halt as u8, 0, 0, 0),
    ];
    let res = vm.run(&code);
    assert!(res.is_ok());
    // Out of bounds read returns empty default buffer, bytes_written = 0
    assert_eq!(vm.registers[1], 0);

    // Direct sgl_io::file_write call
    let written = file_write(&mut vm, "test_file".to_string(), vec![1, 2, 3, 4, 5]);
    assert_eq!(written, 5);
}

#[test]
fn test_adversarial_zero_length_strings() {
    let mut vm = ScriptVm::new();
    let ctx = HostContext::new();
    vm.register_host_context(ctx);
    vm.register_sgl_io();

    // 1. StringConcat of two empty strings
    vm.write_string(covopt_param!("M_215_20", 100), "", true).unwrap();
    vm.write_string(covopt_param!("M_216_20", 200), "", true).unwrap();
    vm.registers[1] = covopt_param!("M_217_22", 999);
    vm.registers[2] = covopt_param!("M_218_22", 5); // StringConcat
    vm.registers[covopt_param!("M_219_17", 3)] = covopt_param!("M_219_22", 100); // R[3]=100 -> arg_reg (str1=R[100], str2=R[101])
    vm.registers[covopt_param!("M_220_17", 100)] = covopt_param!("M_220_24", 100);
    vm.registers[covopt_param!("M_221_17", 101)] = covopt_param!("M_221_24", 200);

    let code1 = [
        Instruction::new(OpCode::SysCall as u8, 1, 2, covopt_param!("M_224_54", 3)),
        Instruction::new(OpCode::Halt as u8, 0, 0, 0),
    ];
    let res1 = vm.run(&code1);
    assert!(res1.is_ok());
    let ptr1 = vm.registers[1] as usize;
    assert!(ptr1 >= 512);
    assert_eq!(vm.read_string(ptr1, None).unwrap(), "");

    // 2. StringLength of empty string
    vm.pc = 0;
    vm.registers[covopt_param!("M_235_17", 10)] = covopt_param!("M_235_23", 999);
    vm.registers[covopt_param!("M_236_17", 11)] = covopt_param!("M_236_23", 6); // StringLength
    vm.registers[covopt_param!("M_237_17", 12)] = covopt_param!("M_237_23", 100);
    let code2 = [
        Instruction::new(OpCode::SysCall as u8, covopt_param!("M_239_48", 10), covopt_param!("M_239_52", 11), covopt_param!("M_239_56", 12)),
        Instruction::new(OpCode::Halt as u8, 0, 0, 0),
    ];
    let res2 = vm.run(&code2);
    assert!(res2.is_ok());
    assert_eq!(vm.registers[10], 0);

    // 3. StringToUpper / StringToLower of empty string
    vm.pc = 0;
    vm.registers[covopt_param!("M_248_17", 20)] = covopt_param!("M_248_23", 999);
    vm.registers[covopt_param!("M_249_17", 21)] = covopt_param!("M_249_23", 8); // StringToUpper
    vm.registers[covopt_param!("M_250_17", 22)] = covopt_param!("M_250_23", 100);
    let code3 = [
        Instruction::new(OpCode::SysCall as u8, covopt_param!("M_252_48", 20), covopt_param!("M_252_52", 21), covopt_param!("M_252_56", 22)),
        Instruction::new(OpCode::Halt as u8, 0, 0, 0),
    ];
    let res3 = vm.run(&code3);
    assert!(res3.is_ok());
    let ptr3 = vm.registers[covopt_param!("M_257_28", 20)] as usize;
    assert!(ptr3 >= 512);
    assert_eq!(vm.read_string(ptr3, None).unwrap(), "");

    // 4. Direct package calls
    assert_eq!(string_concat(&mut vm, "".to_string(), "".to_string()), "");
    assert_eq!(string_length(&mut vm, "".to_string()), 0);
    assert_eq!(string_to_upper(&mut vm, "".to_string()), "");
    assert_eq!(string_to_lower(&mut vm, "".to_string()), "");
}

#[test]
fn test_adversarial_out_of_bounds_slice_indices() {
    let mut vm = ScriptVm::new();
    let ctx = HostContext::new();
    vm.register_host_context(ctx);
    vm.register_sgl_io();

    let sample_str = "Hello, ScriptGo!";
    vm.write_string(covopt_param!("M_276_20", 100), sample_str, true).unwrap();

    // 1. start > end (e.g. start = 10, end = 5)
    vm.registers[1] = covopt_param!("M_279_22", 999);
    vm.registers[2] = covopt_param!("M_280_22", 7); // StringSlice
    vm.registers[covopt_param!("M_281_17", 3)] = covopt_param!("M_281_22", 22); // R[3]=22 -> arg_reg (R[22]=100 (str_addr), R[23]=10 (start), R[24]=5 (end))
    vm.registers[covopt_param!("M_282_17", 22)] = covopt_param!("M_282_23", 100);
    vm.registers[covopt_param!("M_283_17", 23)] = covopt_param!("M_283_23", 10);
    vm.registers[covopt_param!("M_284_17", 24)] = covopt_param!("M_284_23", 5);

    let code1 = [
        Instruction::new(OpCode::SysCall as u8, 1, 2, covopt_param!("M_287_54", 3)),
        Instruction::new(OpCode::Halt as u8, 0, 0, 0),
    ];
    let res1 = vm.run(&code1);
    assert!(res1.is_ok());
    let ptr1 = vm.registers[1] as usize;
    assert!(ptr1 >= 512);
    assert_eq!(vm.read_string(ptr1, None).unwrap(), "");

    // 2. start > len, end > len (e.g. start = 100, end = 200)
    vm.pc = 0;
    vm.registers[covopt_param!("M_298_17", 10)] = covopt_param!("M_298_23", 999);
    vm.registers[covopt_param!("M_299_17", 11)] = covopt_param!("M_299_23", 7);
    vm.registers[covopt_param!("M_300_17", 12)] = covopt_param!("M_300_23", 22);
    vm.registers[covopt_param!("M_301_17", 22)] = covopt_param!("M_301_23", 100);
    vm.registers[covopt_param!("M_302_17", 23)] = covopt_param!("M_302_23", 100);
    vm.registers[covopt_param!("M_303_17", 24)] = covopt_param!("M_303_23", 200);
    let code2 = [
        Instruction::new(OpCode::SysCall as u8, covopt_param!("M_305_48", 10), covopt_param!("M_305_52", 11), covopt_param!("M_305_56", 12)),
        Instruction::new(OpCode::Halt as u8, 0, 0, 0),
    ];
    let res2 = vm.run(&code2);
    assert!(res2.is_ok());
    let ptr2 = vm.registers[covopt_param!("M_310_28", 10)] as usize;
    assert!(ptr2 >= 512);
    assert_eq!(vm.read_string(ptr2, None).unwrap(), "");

    // 3. start = 0, end = u32::MAX
    vm.pc = 0;
    vm.registers[covopt_param!("M_316_17", 20)] = covopt_param!("M_316_23", 999);
    vm.registers[covopt_param!("M_317_17", 21)] = covopt_param!("M_317_23", 7);
    vm.registers[covopt_param!("M_318_17", 22)] = covopt_param!("M_318_23", 22);
    vm.registers[covopt_param!("M_319_17", 22)] = covopt_param!("M_319_23", 100);
    vm.registers[covopt_param!("M_320_17", 23)] = 0;
    vm.registers[covopt_param!("M_321_17", 24)] = u32::MAX;
    let code3 = [
        Instruction::new(OpCode::SysCall as u8, covopt_param!("M_323_48", 20), covopt_param!("M_323_52", 21), covopt_param!("M_323_56", 22)),
        Instruction::new(OpCode::Halt as u8, 0, 0, 0),
    ];
    let res3 = vm.run(&code3);
    assert!(res3.is_ok());
    let ptr3 = vm.registers[covopt_param!("M_328_28", 20)] as usize;
    assert!(ptr3 >= 512);
    assert_eq!(vm.read_string(ptr3, None).unwrap(), sample_str);

    // 4. Unicode multi-byte characters slicing
    let unicode_str = "🦀🚀ScriptGo🔥⚡";
    assert_eq!(string_slice(&mut vm, unicode_str.to_string(), 0, 2), "🦀🚀");
    assert_eq!(string_slice(&mut vm, unicode_str.to_string(), 2, 10), "ScriptGo");
    assert_eq!(string_slice(&mut vm, unicode_str.to_string(), 10, 12), "🔥⚡");
    assert_eq!(string_slice(&mut vm, unicode_str.to_string(), 50, 100), "");
    assert_eq!(string_slice(&mut vm, unicode_str.to_string(), 10, 5), "");
}

#[test]
fn test_adversarial_closed_and_invalid_sockets() {
    let mut vm = ScriptVm::new();
    let ctx = HostContext::new();
    vm.register_host_context(ctx);
    vm.register_sgl_net();

    // 1. SocketSend to non-existent socket_id = 9999
    vm.write_bytes(covopt_param!("M_349_19", 100), b"data_to_send").unwrap();
    vm.registers[1] = covopt_param!("M_350_22", 999); // dest
    vm.registers[2] = covopt_param!("M_351_22", 4);   // SocketSend
    vm.registers[covopt_param!("M_352_17", 3)] = covopt_param!("M_352_22", 200); // R[3]=200 -> arg_reg (R[200]=9999 (socket_id), R[201]=100 (buf_addr), R[202]=12 (buf_len))
    vm.registers[covopt_param!("M_353_17", 200)] = covopt_param!("M_353_24", 9999);
    vm.registers[covopt_param!("M_354_17", 201)] = covopt_param!("M_354_24", 100);
    vm.registers[covopt_param!("M_355_17", 202)] = covopt_param!("M_355_24", 12);

    let code1 = [
        Instruction::new(OpCode::HardwareCall as u8, 1, 2, covopt_param!("M_358_59", 3)),
        Instruction::new(OpCode::Halt as u8, 0, 0, 0),
    ];
    let res1 = vm.run(&code1);
    assert!(res1.is_ok());
    assert_eq!(vm.registers[1], 0);

    // 2. SocketRecv from non-existent socket_id = 9999
    vm.pc = 0;
    vm.registers[covopt_param!("M_367_17", 10)] = covopt_param!("M_367_23", 999);
    vm.registers[covopt_param!("M_368_17", 11)] = covopt_param!("M_368_23", 5);   // SocketRecv
    vm.registers[covopt_param!("M_369_17", 12)] = covopt_param!("M_369_23", 200); // R[12]=200 -> arg_reg (R[200]=9999, R[201]=300 (dest_buf_addr))
    vm.registers[covopt_param!("M_370_17", 200)] = covopt_param!("M_370_24", 9999);
    vm.registers[covopt_param!("M_371_17", 201)] = covopt_param!("M_371_24", 300);

    let code2 = [
        Instruction::new(OpCode::HardwareCall as u8, covopt_param!("M_374_53", 10), covopt_param!("M_374_57", 11), covopt_param!("M_374_61", 12)),
        Instruction::new(OpCode::Halt as u8, 0, 0, 0),
    ];
    let res2 = vm.run(&code2);
    assert!(res2.is_ok());
    assert_eq!(vm.registers[10], 0);

    // 3. Direct package calls for sgl_net
    assert_eq!(socket_send(&mut vm, 9999, vec![1, 2, 3]), 0);
    assert_eq!(socket_recv(&mut vm, 9999, 100), 0);

    // Connect socket 1, then test recv on empty buffer
    let sock1 = socket_connect(&mut vm, "127.0.0.1:8080".to_string());
    assert_eq!(sock1, 1);
    assert_eq!(socket_recv(&mut vm, sock1, 100), 0);
}

#[test]
fn test_adversarial_invalid_registers_and_wrapping() {
    let mut vm = ScriptVm::new();
    let ctx = HostContext::new();
    vm.register_host_context(ctx);
    vm.register_sgl_io();
    vm.register_sgl_net();

    // 1. arg_reg = 255: wrapping addition for sub-arguments in R[255], R[0], R[1]
    // FileWrite with Instruction arg_c = 255 -> R[255]=100 (path_addr), R[0]=200 (data_addr), R[1]=13 (data_len)
    vm.write_string(covopt_param!("M_401_20", 100), "wrapping.txt", true).unwrap();
    vm.write_bytes(covopt_param!("M_402_19", 200), b"wrapping_data").unwrap();
    vm.registers[covopt_param!("M_403_17", 255)] = covopt_param!("M_403_24", 100);
    vm.registers[0] = covopt_param!("M_404_22", 200);
    vm.registers[1] = covopt_param!("M_405_22", 13);

    vm.registers[covopt_param!("M_407_17", 10)] = covopt_param!("M_407_23", 999); // dest
    vm.registers[covopt_param!("M_408_17", 11)] = 2;   // FileWrite

    let code1 = [
        Instruction::new(OpCode::SysCall as u8, covopt_param!("M_411_48", 10), covopt_param!("M_411_52", 11), covopt_param!("M_411_56", 255)),
        Instruction::new(OpCode::Halt as u8, 0, 0, 0),
    ];
    let res1 = vm.run(&code1);
    assert!(res1.is_ok());
    assert_eq!(vm.registers[10], 13);

    // Verify written content
    let read_back = file_read(&mut vm, "wrapping.txt".to_string()).unwrap();
    assert_eq!(read_back, "wrapping_data");

    // 2. Unknown SysCall command ID (e.g. 255)
    vm.pc = 0;
    vm.registers[covopt_param!("M_424_17", 20)] = covopt_param!("M_424_23", 999);
    vm.registers[covopt_param!("M_425_17", 21)] = covopt_param!("M_425_23", 255); // Unknown
    vm.registers[covopt_param!("M_426_17", 22)] = 0;
    let code2 = [
        Instruction::new(OpCode::SysCall as u8, covopt_param!("M_428_48", 20), covopt_param!("M_428_52", 21), covopt_param!("M_428_56", 22)),
        Instruction::new(OpCode::Halt as u8, 0, 0, 0),
    ];
    let res2 = vm.run(&code2);
    assert!(res2.is_ok());
    assert_eq!(vm.registers[20], 0);

    // 3. Unknown HardwareCall command ID (e.g. 255)
    vm.pc = 0;
    vm.registers[covopt_param!("M_437_17", 30)] = covopt_param!("M_437_23", 999);
    vm.registers[covopt_param!("M_438_17", 31)] = covopt_param!("M_438_23", 255); // Unknown
    vm.registers[covopt_param!("M_439_17", 32)] = 0;
    let code3 = [
        Instruction::new(OpCode::HardwareCall as u8, covopt_param!("M_441_53", 30), covopt_param!("M_441_57", 31), covopt_param!("M_441_61", 32)),
        Instruction::new(OpCode::Halt as u8, 0, 0, 0),
    ];
    let res3 = vm.run(&code3);
    assert!(res3.is_ok());
    assert_eq!(vm.registers[30], 0);
}

#[test]
fn test_adversarial_environment_variables() {
    let mut vm = ScriptVm::new();
    let ctx = HostContext::new();
    vm.register_host_context(ctx);
    vm.register_sgl_io();

    // 1. GetEnv with non-existent variable key
    vm.write_string(covopt_param!("M_457_20", 100), "NON_EXISTENT_KEY_12345", true).unwrap();
    vm.registers[1] = covopt_param!("M_458_22", 999);
    vm.registers[2] = covopt_param!("M_459_22", 4); // GetEnv
    vm.registers[covopt_param!("M_460_17", 3)] = covopt_param!("M_460_22", 100);
    let code1 = [
        Instruction::new(OpCode::SysCall as u8, 1, 2, covopt_param!("M_462_54", 3)),
        Instruction::new(OpCode::Halt as u8, 0, 0, 0),
    ];
    let res1 = vm.run(&code1);
    assert!(res1.is_ok());
    assert_eq!(vm.registers[1], 0);

    // 2. Direct get_env call with empty key
    assert_eq!(get_env(&mut vm, "".to_string()), Err(400));
    assert_eq!(get_env(&mut vm, "MISSING".to_string()), Err(404));
    assert_eq!(get_env(&mut vm, "USER".to_string()), Ok("sgl_runner".to_string()));
}
