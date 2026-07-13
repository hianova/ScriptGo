use tauri::Manager;

struct WatcherState {
    abort_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
    thread_handle: std::sync::Mutex<Option<std::thread::JoinHandle<()>>>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let abort_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let state = std::sync::Arc::new(WatcherState {
        abort_flag: abort_flag.clone(),
        thread_handle: std::sync::Mutex::new(None),
    });

    let app = tauri::Builder::default()
        .manage(state.clone())
        .setup(move |app| {
            // Part 3: DX - Hot Reload Watcher (Polling for simplicity in template)
            #[cfg(debug_assertions)]
            {
                let app_handle = app.handle().clone();
                let abort_clone = abort_flag.clone();
                let handle = std::thread::Builder::new().name("hot-reload-watcher".into()).spawn(move || {
                    let mut last_modified = std::fs::metadata("ui/index.html")
                        .and_then(|m| m.modified())
                        .unwrap_or(std::time::SystemTime::now());
                    
                    while !abort_clone.load(std::sync::atomic::Ordering::Relaxed) {
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
                }).unwrap();
                *state.thread_handle.lock().unwrap() = Some(handle);
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(move |app_handle, event| {
        if let tauri::RunEvent::ExitRequested { .. } = event {
            #[cfg(debug_assertions)]
            {
                let run_state = app_handle.state::<std::sync::Arc<WatcherState>>();
                run_state.abort_flag.store(true, std::sync::atomic::Ordering::SeqCst);
                let mut guard = run_state.thread_handle.lock().unwrap();
                if let Some(h) = guard.take() {
                    let _ = h.join();
                }
            }
        }
    });
}