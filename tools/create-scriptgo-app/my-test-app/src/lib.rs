#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // Part 3: DX - Hot Reload Watcher (Polling for simplicity in template)
            #[cfg(debug_assertions)]
            {
                let app_handle = app.handle();
                std::thread::spawn(move || {
                    let mut last_modified = std::fs::metadata("ui/index.html")
                        .and_then(|m| m.modified())
                        .unwrap_or(std::time::SystemTime::now());
                    
                    loop {
                        std::thread::sleep(std::time::Duration::from_millis(300));
                        if let Ok(meta) = std::fs::metadata("ui/index.html") {
                            if let Ok(modified) = meta.modified() {
                                if modified > last_modified {
                                    last_modified = modified;
                                    let _ = app_handle.emit_all("hot-reload", ());
                                }
                            }
                        }
                    }
                });
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}