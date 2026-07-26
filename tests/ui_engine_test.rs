#![allow(unused_imports)]
use covopt_macro::covopt_param;
use std::io::Write;
use script_go::compiler::codegen::CodeGen;
use script_go::compiler::lexer::Lexer;
use script_go::compiler::parser::Parser;
use script_go::instruction::{Instruction, OpCode};
use script_go::ui_engine::{StreamBuffer, UiDispatcher};
use script_go::vm::{ScriptVm, VmResult};

#[test]
fn test_ui_engine_sgo_script_execution() {
    let source_code = std::fs::read_to_string("tests/ui_sample.sgo")
        .expect("Failed to read tests/ui_sample.sgo");

    let mut lexer = Lexer::new(&source_code);
    let tokens = lexer.tokenize().expect("Lexer tokenization failed");
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("Parser parsing failed");

    let mut codegen = CodeGen::new();
    let bytecode = codegen.compile(&program).expect("CodeGen compilation failed");

    let mut vm = ScriptVm::new();
    let dispatcher = UiDispatcher::new();
    vm.register_ui_dispatcher(dispatcher);

    let result = vm.run(&bytecode).expect("VM execution failed");
    assert!(matches!(result, VmResult::Halted(_)));

    let ui_dispatcher = vm.get_ui_dispatcher().expect("UiDispatcher missing from VM");

    assert_eq!(ui_dispatcher.mount_count, 1, "Mount count should be 1");
    assert_eq!(ui_dispatcher.event_signal_count, 1, "Event signal count should be 1");
    assert_eq!(ui_dispatcher.prop_update_count, 1, "Prop update count should be 1");
    assert_eq!(ui_dispatcher.render_count, 1, "Render count should be 1");

    assert!(ui_dispatcher.components.contains_key(&1), "Component 1 should be mounted");
    assert_eq!(ui_dispatcher.event_queue.len(), 1, "One IPC event signal should be queued");
    assert!(ui_dispatcher.last_rendered_output.is_some(), "Rendered output should be produced");
}

#[test]
fn test_ui_engine_cmd_lifecycle_and_state_sync() {
    let mut vm = ScriptVm::new();
    let dispatcher = UiDispatcher::new();
    vm.register_ui_dispatcher(dispatcher);

    // Setup string memory for component props and event channels
    let prop_addr = covopt_param!("M_52_20", 64);
    let prop_str = "label=Submit, color=blue, enabled=true";
    let _ = vm.write_string(prop_addr, prop_str, true);

    let channel_addr = covopt_param!("M_56_23", 128);
    let channel_str = "tauri://ipc/button_click";
    let _ = vm.write_string(channel_addr, channel_str, true);

    // Bytecode instructions triggering OpCode 36 (UiCall)
    // Register 1 = component ID 42
    // Register 2 = cmd (1..=4)
    // Register 3 = payload memory address
    let instructions = vec![
        Instruction::new(OpCode::LoadImm as u8, 1, 42, 0),        // R[1] = 42
        Instruction::new(OpCode::LoadImm as u8, 2, 1, 0),         // R[2] = 1 (Mount)
        Instruction::new(OpCode::LoadImm as u8, 3, 0, 0),         // R[3] = 0
        Instruction::new(OpCode::UiCall as u8, 1, 2, 3),          // UiCall(R[1], R[2], R[3]) -> Mount ID 42

        Instruction::new(OpCode::LoadImm as u8, 2, 2, 0),         // R[2] = 2 (Event Listener)
        Instruction::new(OpCode::LoadImm as u8, 3, 128, 0),       // R[3] = 128 (Channel string address)
        Instruction::new(OpCode::UiCall as u8, 1, 2, 3),          // UiCall(R[1], R[2], R[3]) -> Event Listener

        Instruction::new(OpCode::LoadImm as u8, 2, 3, 0),         // R[2] = 3 (Prop Update)
        Instruction::new(OpCode::LoadImm as u8, 3, 64, 0),        // R[3] = 64 (Prop string address)
        Instruction::new(OpCode::UiCall as u8, 1, 2, 3),          // UiCall(R[1], R[2], R[3]) -> Prop Update

        Instruction::new(OpCode::LoadImm as u8, 2, 4, 0),         // R[2] = 4 (Render)
        Instruction::new(OpCode::LoadImm as u8, 3, 0, 0),         // R[3] = 0
        Instruction::new(OpCode::UiCall as u8, 1, 2, 3),          // UiCall(R[1], R[2], R[3]) -> Render

        Instruction::new(OpCode::Halt as u8, 0, 0, 0),
    ];

    let result = vm.run(&instructions).expect("VM execution failed");
    assert!(matches!(result, VmResult::Halted(_)));

    let ui_dispatcher = vm.get_ui_dispatcher().expect("UiDispatcher missing from VM");

    assert_eq!(ui_dispatcher.mount_count, 1);
    assert_eq!(ui_dispatcher.event_signal_count, 1);
    assert_eq!(ui_dispatcher.prop_update_count, 1);
    assert_eq!(ui_dispatcher.render_count, 1);

    let component = ui_dispatcher.components.get(&covopt_param!("M_95_50", 42)).expect("Component 42 missing");
    assert_eq!(component.id, 42);
    assert_eq!(component.props.get("label").map(|s| s.as_str()), Some("Submit"));
    assert_eq!(component.props.get("color").map(|s| s.as_str()), Some("blue"));
    assert_eq!(component.props.get("enabled").map(|s| s.as_str()), Some("true"));
    assert!(component.event_listeners.contains(&channel_str.to_string()));

    let event = &ui_dispatcher.event_queue[0];
    assert_eq!(event.component_id, 42);
    assert_eq!(event.channel, channel_str);

    let rendered_output = ui_dispatcher.last_rendered_output.as_ref().unwrap();
    assert!(rendered_output.contains("component_id=\"42\""));
    assert!(rendered_output.contains("label=\"Submit\""));
}

#[test]
fn test_ui_engine_large_document_streaming_100mb() {
    let mut dispatcher = UiDispatcher::new();

    // Generate simulated 100MB Markdown document in chunks
    // 100MB = 100 * 1024 * 1024 bytes = 104,857,600 bytes
    let chunk_size = covopt_param!("M_117_21", 10) * covopt_param!("M_117_26", 1024) * covopt_param!("M_117_33", 1024); // 10MB chunk size
    let total_chunks = covopt_param!("M_118_23", 10);
    let expected_total_bytes = chunk_size * total_chunks;

    let sample_markdown_chunk = vec![b'#'; chunk_size];

    for chunk_index in 0..total_chunks {
        dispatcher.stream_buffer.append_chunk(&sample_markdown_chunk);
        assert_eq!(dispatcher.stream_buffer.chunk_count(), chunk_index + 1);
    }

    assert_eq!(
        dispatcher.stream_buffer.total_bytes, expected_total_bytes,
        "StreamBuffer total_bytes must equal 100MB (104,857,600 bytes)"
    );
    assert_eq!(dispatcher.stream_buffer.chunk_count(), total_chunks);

    // Verify stream assembly
    let assembled_data = dispatcher.stream_buffer.assemble();
    assert_eq!(assembled_data.len(), expected_total_bytes);
    assert_eq!(&assembled_data[..100], &sample_markdown_chunk[..100]);

    // Verify rendering output includes stream buffer status
    let vm = ScriptVm::new();
    dispatcher.dispatch(&vm, 1, covopt_param!("M_141_32", 4), 0).expect("Render dispatch failed");
    let render_output = dispatcher.last_rendered_output.as_ref().unwrap();

    assert!(
        render_output.contains(&format!("total_bytes=\"{}\"", expected_total_bytes)),
        "Rendered XML frame output should report 100MB total bytes"
    );
}

#[test]
fn test_ui_engine_stream_buffer_operations() {
    let mut buffer = StreamBuffer::new();
    assert_eq!(buffer.total_bytes, 0);
    assert_eq!(buffer.chunk_count(), 0);

    let chunk1 = b"Hello, ";
    let chunk2 = b"ScriptGo Tauri UI Engine!";
    buffer.append_chunk(chunk1);
    buffer.append_chunk(chunk2);

    assert_eq!(buffer.chunk_count(), 2);
    assert_eq!(buffer.total_bytes, chunk1.len() + chunk2.len());

    let assembled = buffer.assemble();
    assert_eq!(assembled, b"Hello, ScriptGo Tauri UI Engine!");

    buffer.clear();
    assert_eq!(buffer.total_bytes, 0);
    assert_eq!(buffer.chunk_count(), 0);
}
