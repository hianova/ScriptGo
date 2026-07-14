use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: create-scriptgo-app <project-name>");
        std::process::exit(1);
    }

    let project_name = &args[1];
    let project_dir = Path::new(project_name);

    if project_dir.exists() {
        eprintln!("Directory {} already exists!", project_name);
        std::process::exit(1);
    }

    // Create directories
    fs::create_dir_all(project_dir.join("src")).unwrap();
    fs::create_dir_all(project_dir.join("ui")).unwrap();

    // 1. Cargo.toml
    let cargo_toml = format!(
        r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[build-dependencies]
tauri-build = {{ version = "2.0.0-rc", features = [] }}

[dependencies]
tauri = {{ version = "2.0.0-rc", features = [] }}
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
script_go = {{ git = "https://github.com/hianova/ScriptGo" }}
"#,
        project_name
    );
    fs::write(project_dir.join("Cargo.toml"), cargo_toml).unwrap();

    // 2. build.rs
    fs::write(
        project_dir.join("build.rs"),
        "fn main() { tauri_build::build() }",
    )
    .unwrap();

    // 3. tauri.conf.json
    let tauri_conf = r#"{
  "build": { "devPath": "ui", "distDir": "ui" },
  "package": { "productName": "ScriptGoApp", "version": "0.1.0" },
  "tauri": { "bundle": { "active": false, "identifier": "com.scriptgo.app" } }
}"#;
    fs::write(project_dir.join("tauri.conf.json"), tauri_conf).unwrap();

    // 4. ui/index.html (Vanilla JS + Virtual Scroll + Hot Reload Listener)
    let index_html = r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>ScriptGo App</title>
    <style>
        body { font-family: sans-serif; background: #121212; color: #fff; text-align: center; margin-top: 50px; }
        h1 { color: #00ffcc; }
    </style>
</head>
<body>
    <h1>🚀 Welcome to ScriptGo + Tauri!</h1>
    <p>Zero-allocation backend. Ultra-thin frontend.</p>
    <p>Edit <code>ui/index.html</code> to see instant hot-reloads!</p>
    <script>
        // DX: Auto Hot-Reload Listener
        const { listen } = window.__TAURI__.event;
        listen('hot-reload', () => {
            console.log("Hot reload triggered!");
            location.reload();
        });
    </script>
</body>
</html>"#;
    fs::write(project_dir.join("ui/index.html"), index_html).unwrap();

    // 5. src/lib.rs (Backend with File Watcher for DX)
    let main_rs = r#"use tauri::Manager;

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
                                    let _ = app_handle.emit("hot-reload", ());
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
}"#;
    fs::write(project_dir.join("src/lib.rs"), main_rs).unwrap();

    // 6. src/main.rs (CLI entry)
    let cli_main = r#"fn main() {
    create_scriptgo_app::run();
}"#;
    // Actually the package name might be hyphens, so module is underscores
    let pkg_module = project_name.replace("-", "_");
    fs::write(
        project_dir.join("src/main.rs"),
        cli_main.replace("create_scriptgo_app", &pkg_module),
    )
    .unwrap();

    println!("✅ ScriptGo App '{}' created successfully!", project_name);
    println!("👉 Run the following commands:");
    println!("  cd {}", project_name);
    println!("  cargo run");
}
