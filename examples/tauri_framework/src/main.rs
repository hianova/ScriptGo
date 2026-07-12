#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use script_go::assembler::parse_asm;
use script_go::vm::ScriptVm;
use serde::Serialize;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tauri::Manager;

#[derive(Clone, Serialize)]
struct UiCommandPayload {
    arg1: usize,
    arg2: usize,
    arg3: usize,
}

#[derive(Clone, Serialize)]
struct AlertPayload {
    msg: String,
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let app_handle = app.handle();
            
            // Start the ScriptGo VM Engine in a background thread
            let _vm_handle = thread::Builder::new().name("scriptgo-vm".into()).spawn(move || {
                // Wait for frontend to load
                thread::sleep(Duration::from_millis(1500));
                
                let source_code = r#"
                    # Create Root View (ID=10, Type=1)
                    LOADIMM 1 10
                    LOADIMM 2 1
                    UICALL 1 2 0
                    
                    # Append Root View to HTML Body (ID=10, AppendTo=0)
                    LOADIMM 2 4
                    LOADIMM 3 0
                    UICALL 1 2 3
                    
                    # Create Text (ID=20, Type=2, Data=99)
                    LOADIMM 1 20
                    LOADIMM 2 2
                    LOADIMM 3 99
                    UICALL 1 2 3
                    
                    # Append Text to Root View (ID=20, AppendTo=10)
                    LOADIMM 2 4
                    LOADIMM 3 10
                    UICALL 1 2 3
                    
                    # Create Button (ID=30, Type=3)
                    LOADIMM 1 30
                    LOADIMM 2 3
                    UICALL 1 2 0
                    
                    # Append Button to Root View (ID=30, AppendTo=10)
                    LOADIMM 2 4
                    LOADIMM 3 10
                    UICALL 1 2 3
                    
                    HALT
                "#;
                
                let code = parse_asm(source_code).expect("Failed to parse script");
                
                let mut vm = ScriptVm::new();
                
                // Bind the UI Handler to emit Tauri IPC events
                let handle = app_handle.clone();
                vm.ui_handler = Some(Arc::new(move |arg1, arg2, arg3| {
                    handle.emit_all("ui-cmd", UiCommandPayload { arg1, arg2, arg3 }).unwrap();
                    // Small sleep for visual effect in this demo
                    thread::sleep(Duration::from_millis(300));
                }));
                
                println!("🚀 Launching ScriptGo UI Engine...");
                if let Err(e) = vm.run(&code) {
                    app_handle.emit_all("ui-alert", AlertPayload { msg: format!("VM Error: {:?}", e) }).unwrap();
                } else {
                    println!("✅ ScriptGo Execution Finished.");
                }
            });
            
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
