use pulldown_cmark::Parser;
use script_go::assembler::parse_asm;
use script_go::vm::{ScriptVm, TraceStep};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::sync::Mutex;
use std::time::Instant;
use tauri::Manager;
use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotePayload {
    id: usize,
    title: String,
    content: String,
}

#[derive(Debug, Clone)]
struct Note {
    id: usize,
    title: String,
    content: String,
    title_lower: String,
    content_lower: String,
}

pub mod wasm_plugin;

struct AppState {
    vm: Mutex<ScriptVm>,
    notes: Mutex<Vec<Note>>,
    plugins: Mutex<HashMap<String, wasm_plugin::WasmPlugin>>,
}

#[tauri::command]
fn render_plugin(name: String, state: tauri::State<AppState>) -> Result<String, String> {
    let plugins = state.plugins.lock().unwrap();
    if let Some(plugin) = plugins.get(&name) {
        plugin.render().map_err(|e| e.to_string())
    } else {
        Err("Plugin not found".into())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DevToolsStatePayload {
    pub registers: Vec<u32>,
    pub pc: usize,
    pub memory: Vec<u8>,
    pub trace: Vec<TraceStep>,
}

#[tauri::command]
fn get_vm_devtools_state(state: tauri::State<AppState>) -> DevToolsStatePayload {
    let vm = state.vm.lock().unwrap();
    let mut trace_vec = Vec::with_capacity(vm.trace_count);
    let mut idx = (vm.trace_head + 1024 - vm.trace_count) % 1024;
    for _ in 0..vm.trace_count {
        trace_vec.push(vm.trace_buffer[idx]);
        idx = (idx + 1) % 1024;
    }
    DevToolsStatePayload {
        registers: vm.registers.to_vec(),
        pc: vm.pc,
        memory: vm.memory.to_vec(),
        trace: trace_vec,
    }
}

#[tauri::command]
fn export_vm_trace(state: tauri::State<AppState>) -> Result<String, String> {
    let vm = state.vm.lock().unwrap();
    let mut trace_vec = Vec::with_capacity(vm.trace_count);
    let mut idx = (vm.trace_head + 1024 - vm.trace_count) % 1024;
    for _ in 0..vm.trace_count {
        trace_vec.push(vm.trace_buffer[idx]);
        idx = (idx + 1) % 1024;
    }
    let json = serde_json::to_string_pretty(&trace_vec).map_err(|e| e.to_string())?;
    std::fs::write("logic_trace.trace", &json).map_err(|e| e.to_string())?;
    Ok("Saved to logic_trace.trace".into())
}

#[tauri::command]
fn open_devtools(app_handle: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app_handle.get_webview_window("devtools") {
        let _ = window.show();
        let _ = window.set_focus();
    } else {
        tauri::WebviewWindowBuilder::new(
            &app_handle,
            "devtools",
            tauri::WebviewUrl::App("devtools.html".into()),
        )
        .title("ScriptGo DevTools")
        .inner_size(800.0, 600.0)
        .build()
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn search_notes(query: String, state: tauri::State<AppState>) -> Vec<NotePayload> {
    // 驗收關卡四：OTA 熱更新 (Zero-Downtime)
    let sgo_code = fs::read_to_string("logic.sgo").unwrap_or_else(|_| "HALT".to_string());
    if let Ok(instructions) = parse_asm(&sgo_code) {
        let mut vm = state.vm.lock().unwrap();
        vm.tracing_enabled = true; // Enable trace logging so we can inspect it in DevTools

        // Wrap execution in catch_unwind to handle any potential panic inside custom hooks or VM
        let vm_ref = &mut *vm;
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            vm_ref.run(&instructions)
        }));

        match result {
            Ok(Ok(_)) => {}
            Ok(Err(e)) => {
                println!("[VM ERROR]: {:?}", e);
            }
            Err(_) => {
                println!("[VM PANICKED] Let it crash - Recreating VM instance to recover.");
                *vm = ScriptVm::new();
            }
        }
    }

    let notes = state.notes.lock().unwrap();
    let q = query.to_lowercase();

    if q.is_empty() {
        return notes
            .iter()
            .map(|n| NotePayload {
                id: n.id,
                title: n.title.clone(),
                content: n.content.clone(),
            })
            .collect();
    }

    notes
        .iter()
        .filter(|n| n.title_lower.contains(&q) || n.content_lower.contains(&q))
        .map(|n| NotePayload {
            id: n.id,
            title: n.title.clone(),
            content: n.content.clone(),
        })
        .collect()
}

#[tauri::command]
fn fetch_mega_note() -> Vec<u8> {
    let mut mega_doc = String::with_capacity(105 * 1024 * 1024); // ~100MB
    for i in 0..1_000_000 {
        mega_doc.push_str(&format!("Line {}: This is a massive markdown line with **bold** text and [links](http://example.com) and `code`.\n", i));
    }

    // Boss 1: Measure parsing 100MB to AST
    let start = Instant::now();
    let parser = Parser::new(&mega_doc);
    let _ast: Vec<_> = parser.collect();
    println!(
        "✅ Boss 1 (Rust AST): Parsed 100MB Markdown in {:?}",
        start.elapsed()
    );

    // Return raw binary (Tauri 2 IPC optimized binary payload)
    mega_doc.into_bytes()
}

#[tauri::command]
fn parse_real_markdown(md: String) -> String {
    let parser = Parser::new(&md);
    let mut html_output = String::new();
    pulldown_cmark::html::push_html(&mut html_output, parser);
    html_output
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    println!("🚀 [ScriptGo] Generating 10,000 Markdown Notes in memory...");
    let start = Instant::now();
    let mut notes = Vec::with_capacity(10000);
    for i in 1..=10000 {
        let title = format!("Markdown Note #{} - Performance Audit", i);
        let content = format!("This is the auto-generated content for note {}.\n\n# Header\n- Rust backend\n- High Performance IPC\n- Virtual DOM scrolling\n\nScriptGo VM is processing this at ultra-low latency.", i);
        notes.push(Note {
            id: i,
            title_lower: title.to_lowercase(),
            content_lower: content.to_lowercase(),
            title,
            content,
        });
    }
    println!("✅ Generated 10,000 notes in {:?}", start.elapsed());

    let vm = ScriptVm::new();
    let app_state = AppState {
        vm: Mutex::new(vm),
        notes: Mutex::new(notes),
        plugins: Mutex::new(HashMap::new()),
    };

    tauri::Builder::default()
        .manage(app_state)
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();
            #[cfg(target_os = "macos")]
            apply_vibrancy(&window, NSVisualEffectMaterial::Sidebar, None, None).unwrap_or(());
            
            #[cfg(target_os = "windows")]
            apply_mica(&window, None).unwrap_or(());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            search_notes,
            fetch_mega_note,
            parse_real_markdown,
            render_plugin,
            get_vm_devtools_state,
            export_vm_trace,
            open_devtools
        ])
        .on_window_event(|window, event| if let tauri::WindowEvent::DragDrop(tauri::DragDropEvent::Drop { paths, .. }) = event {
            if let Some(path) = paths.first() {
                let file_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                if let Ok(bytes) = std::fs::read(path) {
                    let start = std::time::Instant::now();
                    
                    if path.extension().and_then(|s| s.to_str()) == Some("wasm") {
                        // Wasm Plugin Loading
                        if let Ok(plugin) = wasm_plugin::WasmPlugin::new(&bytes) {
                            if let Ok(name) = plugin.get_name() {
                                let elapsed = start.elapsed().as_millis();
                                
                                // Store plugin
                                let state: tauri::State<AppState> = window.state();
                                state.plugins.lock().unwrap().insert(name.clone(), plugin);
                                
                                // Escape for Javascript
                                let name_js = serde_json::to_string(&name).unwrap();
                                let js = format!("
                                    if (window.addWasmPlugin) window.addWasmPlugin({});
                                    document.getElementById('perf-monitor').innerText = 'Loaded Wasm plugin in {}ms';
                                ", name_js, elapsed);
                                if let Some(webview) = window.get_webview_window("main") {
                                    let _ = webview.eval(&js);
                                }
                            }
                        }
                        return;
                    }

                    // Markdown parsing
                    let text = match String::from_utf8(bytes.clone()) {
                        Ok(s) => s,
                        Err(_) => {
                            let (cow, _, _) = encoding_rs::BIG5.decode(&bytes);
                            cow.into_owned()
                        }
                    };
                    
                    let parser = pulldown_cmark::Parser::new(&text);
                    let mut html = String::new();
                    pulldown_cmark::html::push_html(&mut html, parser);
                    let elapsed = start.elapsed().as_millis();
                    
                    // Use serde_json to safely escape strings for Javascript eval
                    let html_js = serde_json::to_string(&html).unwrap_or_else(|_| "\"Error\"".to_string());
                    let name_js = serde_json::to_string(&file_name).unwrap_or_else(|_| "\"File\"".to_string());
                    let js = format!(
                        "document.getElementById('note-title-display').innerText = {};
                         document.getElementById('note-content-display').innerHTML = {};
                         document.getElementById('perf-monitor').innerText = 'Rendered via Rust OS Drop in {}ms';",
                        name_js, html_js, elapsed
                    );
                    if let Some(webview) = window.get_webview_window("main") {
                        let _ = webview.eval(&js);
                    }
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boss_3_memory_leak_test() {
        // Simulate TTFP (Time to First Paint data structure prep)
        let start = Instant::now();
        let _app_state = AppState {
            vm: Mutex::new(ScriptVm::new()),
            notes: Mutex::new(vec![]),
            plugins: Mutex::new(HashMap::new()),
        };
        println!(
            "✅ Boss 3 (TTFP): Backend structure ready in {:?}",
            start.elapsed()
        );

        let mut mem_initial = 0;
        // Run 1,000,000 iterations of script logic parsing (Hot Reload Simulation)
        let sgo_code = "LOADIMM 1 200\nLOADIMM 2 200\nSUB 3 1 2\nHALT\n";

        let sim_start = Instant::now();
        for i in 0..1_000_000 {
            if i == 0 {
                mem_initial = get_rss_memory();
            }
            let instructions = parse_asm(sgo_code).unwrap();
            let mut vm = ScriptVm::new();
            let _ = vm.run(&instructions);
        }

        let mem_final = get_rss_memory();
        println!(
            "✅ Boss 3 (Memory Check): Initial {} KB, Final {} KB after 1,000,000 hot-reloads",
            mem_initial, mem_final
        );
        println!(
            "✅ Boss 3 (Speed): 1,000,000 iterations finished in {:?}",
            sim_start.elapsed()
        );
        assert!(mem_final < mem_initial + 5000, "Memory leak detected!"); // Should not grow by >5MB
    }

    fn get_rss_memory() -> usize {
        // macOS memory reading via ps
        let output = std::process::Command::new("ps")
            .args(["-o", "rss=", "-p", &std::process::id().to_string()])
            .output()
            .unwrap();
        let s = String::from_utf8_lossy(&output.stdout);
        s.trim().parse().unwrap_or(0)
    }
}
