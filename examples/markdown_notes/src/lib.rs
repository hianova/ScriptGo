use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::Instant;
use script_go::assembler::parse_asm;
use script_go::vm::ScriptVm;
use std::fs;
use pulldown_cmark::Parser;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Note {
    id: usize,
    title: String,
    content: String,
}

struct AppState {
    notes: Vec<Note>,
}

#[tauri::command]
fn search_notes(query: String, state: tauri::State<Mutex<AppState>>) -> Vec<Note> {
    // 驗收關卡四：OTA 熱更新 (Zero-Downtime)
    let sgo_code = fs::read_to_string("logic.sgo").unwrap_or_else(|_| "HALT".to_string());
    if let Ok(instructions) = parse_asm(&sgo_code) {
        let mut vm = ScriptVm::new();
        let _ = vm.run(&instructions);
    }

    let state = state.lock().unwrap();
    let q = query.to_lowercase();
    
    if q.is_empty() {
        return state.notes.clone();
    }
    
    state.notes.iter()
        .filter(|n| n.title.to_lowercase().contains(&q) || n.content.to_lowercase().contains(&q))
        .cloned()
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
    println!("✅ Boss 1 (Rust AST): Parsed 100MB Markdown in {:?}", start.elapsed());
    
    // Return raw binary (Tauri 2 IPC optimized binary payload)
    mega_doc.into_bytes()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    println!("🚀 [ScriptGo] Generating 10,000 Markdown Notes in memory...");
    let start = Instant::now();
    let mut notes = Vec::with_capacity(10000);
    for i in 1..=10000 {
        notes.push(Note {
            id: i,
            title: format!("Markdown Note #{} - Performance Audit", i),
            content: format!("This is the auto-generated content for note {}.\n\n# Header\n- Rust backend\n- High Performance IPC\n- Virtual DOM scrolling\n\nScriptGo VM is processing this at ultra-low latency.", i),
        });
    }
    println!("✅ Generated 10,000 notes in {:?}", start.elapsed());

    let app_state = Mutex::new(AppState { notes });

    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![search_notes, fetch_mega_note])
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
        let app_state = Mutex::new(AppState { notes: vec![] });
        println!("✅ Boss 3 (TTFP): Backend structure ready in {:?}", start.elapsed());
        
        let mut mem_initial = 0;
        // Run 1,000,000 iterations of script logic parsing (Hot Reload Simulation)
        let sgo_code = "LOADIMM 1 200\nLOADIMM 2 200\nSUB 3 1 2\nHALT\n";
        
        let sim_start = Instant::now();
        for i in 0..1_000_000 {
            if i == 0 { mem_initial = get_rss_memory(); }
            let instructions = parse_asm(&sgo_code).unwrap();
            let mut vm = ScriptVm::new();
            let _ = vm.run(&instructions);
        }
        
        let mem_final = get_rss_memory();
        println!("✅ Boss 3 (Memory Check): Initial {} KB, Final {} KB after 1,000,000 hot-reloads", mem_initial, mem_final);
        println!("✅ Boss 3 (Speed): 1,000,000 iterations finished in {:?}", sim_start.elapsed());
        assert!(mem_final < mem_initial + 5000, "Memory leak detected!"); // Should not grow by >5MB
    }

    fn get_rss_memory() -> usize {
        // macOS memory reading via ps
        let output = std::process::Command::new("ps")
            .args(&["-o", "rss=", "-p", &std::process::id().to_string()])
            .output()
            .unwrap();
        let s = String::from_utf8_lossy(&output.stdout);
        s.trim().parse().unwrap_or(0)
    }
}
