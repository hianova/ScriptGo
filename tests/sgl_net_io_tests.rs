#![allow(unused_imports)]
use covopt_macro::covopt_param;
use std::io::Write;
use script_go::sgl::host_handlers::HostContext;
use script_go::sgl::instruction::{Instruction, OpCode};
use script_go::sgl::vm::ScriptVm;
use script_go::{SglIoRegisterExt, SglNetRegisterExt};

#[test]
fn test_sgl_net_http_get_mock_route() {
    let mut vm = ScriptVm::new();
    let mut ctx = HostContext::new();
    ctx.http_mock_routes.insert(
        "https://api.test/data".to_string(),
        r#"{"status":"ok","value":100}"#.to_string(),
    );
    vm.register_host_context(ctx);
    vm.register_sgl_net();

    // Write URL at address 100
    let url = "https://api.test/data";
    vm.write_string(covopt_param!("M_23_20", 100), url, true).unwrap();

    // R[1] = dest, R[2] = cmd 1 (HttpGet), R[3] = arg 100
    vm.registers[1] = 0;
    vm.registers[2] = 1;
    vm.registers[covopt_param!("M_28_17", 3)] = covopt_param!("M_28_22", 100);

    let inst = Instruction::new(OpCode::HardwareCall as u8, 1, 2, covopt_param!("M_30_66", 3));
    let code = [inst, Instruction::new(OpCode::Halt as u8, 0, 0, 0)];

    let res = vm.run(&code);
    assert!(res.is_ok());

    let res_ptr = vm.registers[1] as usize;
    assert!(res_ptr >= 512);

    let res_str = vm.read_string(res_ptr, None).unwrap();
    assert_eq!(res_str, r#"{"status":"ok","value":100}"#);
}

#[test]
fn test_sgl_net_http_get_default_response() {
    let mut vm = ScriptVm::new();
    vm.register_sgl_net();

    let url = "https://example.com/api";
    vm.write_string(covopt_param!("M_49_20", 100), url, true).unwrap();

    vm.registers[1] = 0;
    vm.registers[2] = 1;
    vm.registers[covopt_param!("M_53_17", 3)] = covopt_param!("M_53_22", 100);

    let inst = Instruction::new(OpCode::HardwareCall as u8, 1, 2, covopt_param!("M_55_66", 3));
    let code = [inst, Instruction::new(OpCode::Halt as u8, 0, 0, 0)];

    let res = vm.run(&code);
    assert!(res.is_ok());

    let res_ptr = vm.registers[1] as usize;
    assert!(res_ptr >= 512);

    let res_str = vm.read_string(res_ptr, None).unwrap();
    assert!(res_str.contains("200 OK"));
    assert!(res_str.contains("https://example.com/api"));
}

#[test]
fn test_sgl_net_http_post() {
    let mut vm = ScriptVm::new();
    vm.register_sgl_net();

    let url = "https://api.test/submit";
    let body = r#"{"name":"scriptgo"}"#;

    vm.write_string(covopt_param!("M_77_20", 100), url, true).unwrap();
    vm.write_string(covopt_param!("M_78_20", 200), body, true).unwrap();

    // R[1] = dest, R[2] = cmd 2 (HttpPost), R[3] = arg_reg (R[3]=100, R[4]=200)
    vm.registers[1] = 0;
    vm.registers[2] = 2;
    vm.registers[covopt_param!("M_83_17", 3)] = covopt_param!("M_83_22", 100);
    vm.registers[covopt_param!("M_84_17", 4)] = covopt_param!("M_84_22", 200);

    let inst = Instruction::new(OpCode::HardwareCall as u8, 1, 2, covopt_param!("M_86_66", 3));
    let code = [inst, Instruction::new(OpCode::Halt as u8, 0, 0, 0)];

    let res = vm.run(&code);
    assert!(res.is_ok());

    let res_ptr = vm.registers[1] as usize;
    assert!(res_ptr >= 512);

    let res_str = vm.read_string(res_ptr, None).unwrap();
    assert!(res_str.contains("posted_to"));
    assert!(res_str.contains("https://api.test/submit"));
}

#[test]
fn test_sgl_net_socket_lifecycle() {
    let mut vm = ScriptVm::new();
    let ctx = HostContext::new();
    vm.register_host_context(ctx);
    vm.register_sgl_net();

    // 1. Connect socket
    let addr = "192.168.1.1:9000";
    vm.write_string(covopt_param!("M_109_20", 100), addr, true).unwrap();

    vm.registers[1] = 0; // dest socket_id
    vm.registers[2] = covopt_param!("M_112_22", 3); // SocketConnect
    vm.registers[covopt_param!("M_113_17", 3)] = covopt_param!("M_113_22", 100);

    let inst_connect = Instruction::new(OpCode::HardwareCall as u8, 1, 2, covopt_param!("M_115_74", 3));
    let code_connect = [inst_connect, Instruction::new(OpCode::Halt as u8, 0, 0, 0)];

    let res1 = vm.run(&code_connect);
    assert!(res1.is_ok());
    let socket_id = vm.registers[1];
    assert!(socket_id > 0);

    // 2. Send data on socket
    let payload = b"ping_payload_bytes";
    vm.write_bytes(covopt_param!("M_125_19", 200), payload).unwrap();

    vm.pc = 0;
    vm.registers[covopt_param!("M_128_17", 10)] = 0; // dest sent count
    vm.registers[covopt_param!("M_129_17", 11)] = covopt_param!("M_129_23", 4); // SocketSend
    vm.registers[covopt_param!("M_130_17", 12)] = socket_id; // arg_idx R[12]=socket_id, R[13]=200, R[14]=18
    vm.registers[covopt_param!("M_131_17", 13)] = covopt_param!("M_131_23", 200);
    vm.registers[covopt_param!("M_132_17", 14)] = payload.len() as u32;

    let inst_send = Instruction::new(OpCode::HardwareCall as u8, covopt_param!("M_134_65", 10), covopt_param!("M_134_69", 11), covopt_param!("M_134_73", 12));
    let code_send = [inst_send, Instruction::new(OpCode::Halt as u8, 0, 0, 0)];

    let res2 = vm.run(&code_send);
    assert!(res2.is_ok());
    assert_eq!(vm.registers[10], payload.len() as u32);

    // Verify buffer in HostContext
    let socket_buf = vm
        .get_host_context()
        .unwrap()
        .sockets
        .get(&socket_id)
        .unwrap()
        .send_buffer
        .clone();
    assert_eq!(socket_buf, payload);

    // 3. Receive data on socket
    // Inject receive data into socket
    vm.get_host_context_mut()
        .unwrap()
        .sockets
        .get_mut(&socket_id)
        .unwrap()
        .receive_buffer
        .extend_from_slice(b"pong_reply");

    vm.pc = 0;
    vm.registers[covopt_param!("M_163_17", 20)] = 0; // dest recv count
    vm.registers[covopt_param!("M_164_17", 21)] = covopt_param!("M_164_23", 5); // SocketRecv
    vm.registers[covopt_param!("M_165_17", 22)] = socket_id; // R[22]=socket_id, R[23]=300 (dest buffer address)
    vm.registers[covopt_param!("M_166_17", 23)] = covopt_param!("M_166_23", 300);

    let inst_recv = Instruction::new(OpCode::HardwareCall as u8, covopt_param!("M_168_65", 20), covopt_param!("M_168_69", 21), covopt_param!("M_168_73", 22));
    let code_recv = [inst_recv, Instruction::new(OpCode::Halt as u8, 0, 0, 0)];

    let res3 = vm.run(&code_recv);
    assert!(res3.is_ok());
    assert_eq!(vm.registers[20], 10); // "pong_reply".len()

    let recv_data = vm.read_bytes(covopt_param!("M_175_34", 300), covopt_param!("M_175_39", 10), false).unwrap();
    assert_eq!(&recv_data, b"pong_reply");
}

#[test]
fn test_sgl_net_network_status() {
    let mut vm = ScriptVm::new();
    let ctx = HostContext::new();
    vm.register_host_context(ctx);
    vm.register_sgl_net();

    vm.registers[1] = covopt_param!("M_186_22", 99); // canary
    vm.registers[2] = covopt_param!("M_187_22", 6); // NetworkStatus

    let inst = Instruction::new(OpCode::HardwareCall as u8, 1, 2, 0);
    let code = [inst, Instruction::new(OpCode::Halt as u8, 0, 0, 0)];

    let res = vm.run(&code);
    assert!(res.is_ok());
    assert_eq!(vm.registers[1], 1);
}

#[test]
fn test_sgl_io_file_read_and_write() {
    let mut vm = ScriptVm::new();
    let ctx = HostContext::new();
    vm.register_host_context(ctx);
    vm.register_sgl_io();

    // 1. Read default file "test.txt"
    let file_path = "test.txt";
    vm.write_string(covopt_param!("M_206_20", 100), file_path, true).unwrap();

    vm.registers[1] = 0;
    vm.registers[2] = 1; // FileRead
    vm.registers[covopt_param!("M_210_17", 3)] = covopt_param!("M_210_22", 100);

    let inst_read = Instruction::new(OpCode::SysCall as u8, 1, 2, covopt_param!("M_212_66", 3));
    let code_read = [inst_read, Instruction::new(OpCode::Halt as u8, 0, 0, 0)];

    let res1 = vm.run(&code_read);
    assert!(res1.is_ok());
    let ptr1 = vm.registers[1] as usize;
    assert!(ptr1 >= 512);

    let content1 = vm.read_string(ptr1, None).unwrap();
    assert_eq!(content1, "Hello SGL Virtual Filesystem!");

    // 2. Write new file "config.json"
    let new_file = "config.json";
    let data = b"{\"mode\":\"fast\"}";
    vm.write_string(covopt_param!("M_226_20", 200), new_file, true).unwrap();
    vm.write_bytes(covopt_param!("M_227_19", 300), data).unwrap();

    vm.pc = 0;
    vm.registers[covopt_param!("M_230_17", 10)] = 0; // dest bytes written
    vm.registers[covopt_param!("M_231_17", 11)] = 2; // FileWrite
    vm.registers[covopt_param!("M_232_17", 12)] = covopt_param!("M_232_23", 200); // arg_idx R[12]=200, R[13]=300, R[14]=15
    vm.registers[covopt_param!("M_233_17", 13)] = covopt_param!("M_233_23", 300);
    vm.registers[covopt_param!("M_234_17", 14)] = data.len() as u32;

    let inst_write = Instruction::new(OpCode::SysCall as u8, covopt_param!("M_236_61", 10), covopt_param!("M_236_65", 11), covopt_param!("M_236_69", 12));
    let code_write = [inst_write, Instruction::new(OpCode::Halt as u8, 0, 0, 0)];

    let res2 = vm.run(&code_write);
    assert!(res2.is_ok());
    assert_eq!(vm.registers[10], data.len() as u32);

    // 3. Read back written file
    vm.pc = 0;
    vm.registers[covopt_param!("M_245_17", 20)] = 0;
    vm.registers[covopt_param!("M_246_17", 21)] = 1; // FileRead
    vm.registers[covopt_param!("M_247_17", 22)] = covopt_param!("M_247_23", 200);

    let inst_read2 = Instruction::new(OpCode::SysCall as u8, covopt_param!("M_249_61", 20), covopt_param!("M_249_65", 21), covopt_param!("M_249_69", 22));
    let code_read2 = [inst_read2, Instruction::new(OpCode::Halt as u8, 0, 0, 0)];

    let res3 = vm.run(&code_read2);
    assert!(res3.is_ok());
    let ptr2 = vm.registers[covopt_param!("M_254_28", 20)] as usize;
    assert!(ptr2 >= 512);
    let content2 = vm.read_string(ptr2, None).unwrap();
    assert_eq!(content2, "{\"mode\":\"fast\"}");
}

#[test]
fn test_sgl_io_system_and_env() {
    let mut vm = ScriptVm::new();
    let ctx = HostContext::new();
    vm.register_host_context(ctx);
    vm.register_sgl_io();

    // 1. GetTimestamp
    vm.registers[1] = 0;
    vm.registers[2] = covopt_param!("M_269_22", 3); // GetTimestamp

    let inst_ts = Instruction::new(OpCode::SysCall as u8, 1, 2, 0);
    let code_ts = [inst_ts, Instruction::new(OpCode::Halt as u8, 0, 0, 0)];

    let res1 = vm.run(&code_ts);
    assert!(res1.is_ok());
    assert!(vm.registers[1] > 0);

    // 2. GetEnv "ENV"
    vm.write_string(covopt_param!("M_279_20", 100), "ENV", true).unwrap();

    vm.pc = 0;
    vm.registers[covopt_param!("M_282_17", 10)] = 0;
    vm.registers[covopt_param!("M_283_17", 11)] = covopt_param!("M_283_23", 4); // GetEnv
    vm.registers[covopt_param!("M_284_17", 12)] = covopt_param!("M_284_23", 100);

    let inst_env = Instruction::new(OpCode::SysCall as u8, covopt_param!("M_286_59", 10), covopt_param!("M_286_63", 11), covopt_param!("M_286_67", 12));
    let code_env = [inst_env, Instruction::new(OpCode::Halt as u8, 0, 0, 0)];

    let res2 = vm.run(&code_env);
    assert!(res2.is_ok());
    let env_ptr = vm.registers[covopt_param!("M_291_31", 10)] as usize;
    assert!(env_ptr >= 512);
    let env_val = vm.read_string(env_ptr, None).unwrap();
    assert_eq!(env_val, "production");
}

#[test]
fn test_sgl_io_string_utilities() {
    let mut vm = ScriptVm::new();
    let ctx = HostContext::new();
    vm.register_host_context(ctx);
    vm.register_sgl_io();

    // 1. StringConcat ("Hello", " World")
    vm.write_string(covopt_param!("M_305_20", 100), "Hello", true).unwrap();
    vm.write_string(covopt_param!("M_306_20", 200), " World", true).unwrap();

    vm.registers[1] = 0;
    vm.registers[2] = covopt_param!("M_309_22", 5); // StringConcat
    vm.registers[covopt_param!("M_310_17", 3)] = covopt_param!("M_310_22", 100);
    vm.registers[covopt_param!("M_311_17", 4)] = covopt_param!("M_311_22", 200);

    let inst_concat = Instruction::new(OpCode::SysCall as u8, 1, 2, covopt_param!("M_313_68", 3));
    let code_concat = [inst_concat, Instruction::new(OpCode::Halt as u8, 0, 0, 0)];

    let res1 = vm.run(&code_concat);
    assert!(res1.is_ok());
    let concat_ptr = vm.registers[1] as usize;
    assert!(concat_ptr >= 512);
    assert_eq!(vm.read_string(concat_ptr, None).unwrap(), "Hello World");

    // 2. StringLength ("Hello World")
    vm.pc = 0;
    vm.registers[covopt_param!("M_324_17", 10)] = 0;
    vm.registers[covopt_param!("M_325_17", 11)] = covopt_param!("M_325_23", 6); // StringLength
    vm.registers[covopt_param!("M_326_17", 12)] = concat_ptr as u32;

    let inst_len = Instruction::new(OpCode::SysCall as u8, covopt_param!("M_328_59", 10), covopt_param!("M_328_63", 11), covopt_param!("M_328_67", 12));
    let code_len = [inst_len, Instruction::new(OpCode::Halt as u8, 0, 0, 0)];

    let res2 = vm.run(&code_len);
    assert!(res2.is_ok());
    assert_eq!(vm.registers[10], 11);

    // 3. StringSlice ("Hello World", 0, 5) -> "Hello"
    vm.pc = 0;
    vm.registers[covopt_param!("M_337_17", 20)] = 0;
    vm.registers[covopt_param!("M_338_17", 21)] = covopt_param!("M_338_23", 7); // StringSlice
    vm.registers[covopt_param!("M_339_17", 22)] = concat_ptr as u32; // R[22]=concat_ptr, R[23]=0, R[24]=5
    vm.registers[covopt_param!("M_340_17", 23)] = 0;
    vm.registers[covopt_param!("M_341_17", 24)] = covopt_param!("M_341_23", 5);

    let inst_slice = Instruction::new(OpCode::SysCall as u8, covopt_param!("M_343_61", 20), covopt_param!("M_343_65", 21), covopt_param!("M_343_69", 22));
    let code_slice = [inst_slice, Instruction::new(OpCode::Halt as u8, 0, 0, 0)];

    let res3 = vm.run(&code_slice);
    assert!(res3.is_ok());
    let slice_ptr = vm.registers[covopt_param!("M_348_33", 20)] as usize;
    assert!(slice_ptr >= 512);
    assert_eq!(vm.read_string(slice_ptr, None).unwrap(), "Hello");

    // 4. StringToUpper ("Hello World") -> "HELLO WORLD"
    vm.pc = 0;
    vm.registers[covopt_param!("M_354_17", 30)] = 0;
    vm.registers[covopt_param!("M_355_17", 31)] = covopt_param!("M_355_23", 8); // StringToUpper
    vm.registers[covopt_param!("M_356_17", 32)] = concat_ptr as u32;

    let inst_upper = Instruction::new(OpCode::SysCall as u8, covopt_param!("M_358_61", 30), covopt_param!("M_358_65", 31), covopt_param!("M_358_69", 32));
    let code_upper = [inst_upper, Instruction::new(OpCode::Halt as u8, 0, 0, 0)];

    let res4 = vm.run(&code_upper);
    assert!(res4.is_ok());
    let upper_ptr = vm.registers[covopt_param!("M_363_33", 30)] as usize;
    assert!(upper_ptr >= 512);
    assert_eq!(vm.read_string(upper_ptr, None).unwrap(), "HELLO WORLD");

    // 5. StringToLower ("Hello World") -> "hello world"
    vm.pc = 0;
    vm.registers[covopt_param!("M_369_17", 40)] = 0;
    vm.registers[covopt_param!("M_370_17", 41)] = covopt_param!("M_370_23", 9); // StringToLower
    vm.registers[covopt_param!("M_371_17", 42)] = concat_ptr as u32;

    let inst_lower = Instruction::new(OpCode::SysCall as u8, covopt_param!("M_373_61", 40), covopt_param!("M_373_65", 41), covopt_param!("M_373_69", 42));
    let code_lower = [inst_lower, Instruction::new(OpCode::Halt as u8, 0, 0, 0)];

    let res5 = vm.run(&code_lower);
    assert!(res5.is_ok());
    let lower_ptr = vm.registers[covopt_param!("M_378_33", 40)] as usize;
    assert!(lower_ptr >= 512);
    assert_eq!(vm.read_string(lower_ptr, None).unwrap(), "hello world");
}

#[test]
fn test_sgl_net_and_io_combined_script_execution() {
    let mut vm = ScriptVm::new();
    let ctx = HostContext::new();
    vm.register_host_context(ctx);

    // Register BOTH packages on ScriptVm
    vm.register_sgl_net();
    vm.register_sgl_io();

    // Prepare inputs:
    // Memory 100: "test.txt"
    // Memory 200: "https://api.sgl.internal/status"
    vm.write_string(covopt_param!("M_396_20", 100), "test.txt", true).unwrap();
    vm.write_string(covopt_param!("M_397_20", 200), "https://api.sgl.internal/status", true).unwrap();

    // Script instructions:
    // 0: SysCall R[1], R[2], R[3]   (FileRead "test.txt" -> R[1])
    // 1: HardwareCall R[4], R[5], R[6] (HttpGet "https://api.sgl.internal/status" -> R[4])
    // 2: SysCall R[7], R[8], R[9]   (StringConcat R[1], R[4] -> R[7])
    // 3: Halt
    vm.registers[1] = 0;
    vm.registers[2] = 1; // FileRead
    vm.registers[covopt_param!("M_406_17", 3)] = covopt_param!("M_406_22", 100);

    vm.registers[covopt_param!("M_408_17", 4)] = 0;
    vm.registers[covopt_param!("M_409_17", 5)] = 1; // HttpGet
    vm.registers[covopt_param!("M_410_17", 6)] = covopt_param!("M_410_22", 200);

    vm.registers[covopt_param!("M_412_17", 8)] = covopt_param!("M_412_22", 5); // StringConcat
    // Note: R[9] and R[10] will be populated after step 0 and 1

    let code = [
        Instruction::new(OpCode::SysCall as u8, 1, 2, covopt_param!("M_416_54", 3)),
        Instruction::new(OpCode::HardwareCall as u8, covopt_param!("M_417_53", 4), covopt_param!("M_417_56", 5), covopt_param!("M_417_59", 6)),
        Instruction::new(OpCode::Halt as u8, 0, 0, 0),
    ];

    let res = vm.run(&code);
    assert!(res.is_ok());

    let file_content_ptr = vm.registers[1] as usize;
    let http_response_ptr = vm.registers[covopt_param!("M_425_41", 4)] as usize;

    assert!(file_content_ptr >= 512);
    assert!(http_response_ptr >= 512);

    let file_content = vm.read_string(file_content_ptr, None).unwrap();
    let http_response = vm.read_string(http_response_ptr, None).unwrap();

    assert_eq!(file_content, "Hello SGL Virtual Filesystem!");
    assert_eq!(http_response, r#"{"status":"online","service":"sgl-runtime"}"#);

    // Now test concat instruction using the pointers from previous results
    vm.pc = 0;
    vm.registers[covopt_param!("M_438_17", 7)] = 0;
    vm.registers[covopt_param!("M_439_17", 8)] = covopt_param!("M_439_22", 5); // StringConcat
    vm.registers[covopt_param!("M_440_17", 9)] = file_content_ptr as u32;
    vm.registers[covopt_param!("M_441_17", 10)] = http_response_ptr as u32;

    let concat_code = [
        Instruction::new(OpCode::SysCall as u8, covopt_param!("M_444_48", 7), covopt_param!("M_444_51", 8), covopt_param!("M_444_54", 9)),
        Instruction::new(OpCode::Halt as u8, 0, 0, 0),
    ];

    let res_concat = vm.run(&concat_code);
    assert!(res_concat.is_ok());

    let combined_ptr = vm.registers[covopt_param!("M_451_36", 7)] as usize;
    assert!(combined_ptr >= 512);
    let combined_str = vm.read_string(combined_ptr, None).unwrap();
    assert_eq!(
        combined_str,
        r#"Hello SGL Virtual Filesystem!{"status":"online","service":"sgl-runtime"}"#
    );
}

#[test]
fn test_invalid_address_safety_guarantee() {
    let mut vm = ScriptVm::new();
    vm.register_sgl_net();
    vm.register_sgl_io();

    // R[1] = dest, R[2] = 1 (HttpGet), R[3] = 0xDEADBEEF
    vm.registers[1] = covopt_param!("M_467_22", 999);
    vm.registers[2] = 1;
    vm.registers[covopt_param!("M_469_17", 3)] = covopt_param!("M_469_22", 3735928559);

    let inst1 = Instruction::new(OpCode::HardwareCall as u8, 1, 2, covopt_param!("M_471_67", 3));
    let code1 = [inst1, Instruction::new(OpCode::Halt as u8, 0, 0, 0)];

    let res1 = vm.run(&code1);
    assert!(res1.is_ok());
    assert_eq!(vm.registers[1], 0);

    // R[10] = dest, R[11] = 1 (FileRead), R[12] = 0xDEADBEEF
    vm.pc = 0;
    vm.registers[covopt_param!("M_480_17", 10)] = covopt_param!("M_480_23", 888);
    vm.registers[covopt_param!("M_481_17", 11)] = 1;
    vm.registers[covopt_param!("M_482_17", 12)] = covopt_param!("M_482_23", 3735928559);

    let inst2 = Instruction::new(OpCode::SysCall as u8, covopt_param!("M_484_56", 10), covopt_param!("M_484_60", 11), covopt_param!("M_484_64", 12));
    let code2 = [inst2, Instruction::new(OpCode::Halt as u8, 0, 0, 0)];

    let res2 = vm.run(&code2);
    assert!(res2.is_ok());
    assert_eq!(vm.registers[10], 0);
}
