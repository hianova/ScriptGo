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
    let cargo_toml = format!(r#"[package]
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
"#, project_name);
    fs::write(project_dir.join("Cargo.toml"), cargo_toml).unwrap();

    // 2. build.rs
    fs::write(project_dir.join("build.rs"), "fn main() { tauri_build::build() }").unwrap();

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
    let main_rs = r#"#[cfg_attr(mobile, tauri::mobile_entry_point)]
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
}"#;
    fs::write(project_dir.join("src/lib.rs"), main_rs).unwrap();

    // 6. src/main.rs (CLI entry)
    let cli_main = r#"fn main() {
    create_scriptgo_app::run();
}"#;
    // Actually the package name might be hyphens, so module is underscores
    let pkg_module = project_name.replace("-", "_");
    fs::write(project_dir.join("src/main.rs"), cli_main.replace("create_scriptgo_app", &pkg_module)).unwrap();

    println!("✅ ScriptGo App '{}' created successfully!", project_name);
    println!("👉 Run the following commands:");
    println!("  cd {}", project_name);
    println!("  cargo run");
}
