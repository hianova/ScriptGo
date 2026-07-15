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
edition = "2024"

[workspace]

[dependencies]
sgl-browser = {{ path = "../sgl-browser" }}
script_go = {{ path = "../ScriptGo" }}
"#,
        project_name
    );
    fs::write(project_dir.join("Cargo.toml"), cargo_toml).unwrap();

    // 2. ui/index.html
    let index_html = r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>ScriptGo App</title>
    <style>
        body { font-family: sans-serif; background: #121212; color: #fff; text-align: center; margin-top: 50px; }
        h1 { color: #00ffcc; }
        .btn { padding: 10px 20px; font-size: 16px; margin: 10px; cursor: pointer; background: #444; color: white; border: none; border-radius: 4px; }
        .btn:hover { background: #666; }
    </style>
</head>
<body>
    <h1>🚀 Welcome to ScriptGo + SglBrowser!</h1>
    <p>Zero-allocation backend. Ultra-thin frontend.</p>
    <button class="btn" onclick="syscall(1, 2, 3)">Test SysCall</button>
</body>
</html>"#;
    fs::write(project_dir.join("ui/index.html"), index_html).unwrap();

    // 3. src/main.rs
    let main_rs = r#"use sgl_browser::builder::SglAppBuilder;
use std::fs;

fn main() {
    let html_content = fs::read_to_string("ui/index.html").unwrap_or_else(|_| {
        "<h1>Error: ui/index.html not found</h1>".to_string()
    });

    SglAppBuilder::new()
        .with_title("ScriptGo App")
        .on_syscall(|a, b, c| {
            println!("Received SysCall from UI: a={}, b={}, c={}", a, b, c);
        })
        .run(&html_content);
}
"#;
    fs::write(project_dir.join("src/main.rs"), main_rs).unwrap();

    println!("✅ ScriptGo App '{}' created successfully using SglBrowser!", project_name);
    println!("👉 Run the following commands:");
    println!("  cd {}", project_name);
    println!("  cargo run");
}
