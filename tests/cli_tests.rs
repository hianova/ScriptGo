#![allow(unused_imports)]
use covopt_macro::covopt_param;
use std::io::Write;
use script_go::cli::{check_cmd, compile_cmd, compile_source, run_cmd, watch_cmd, CliConfig, SubCommand};
use script_go::vm::VmResult;
use std::fs;
use std::process::Command;

#[test]
fn test_cli_arg_parser_subcommands() {
    // 1. Run subcommand
    let args_run = vec!["sgo".to_string(), "run".to_string(), "script.sgo".to_string()];
    let config_run = CliConfig::parse_args(&args_run).expect("Parse run failed");
    assert_eq!(
        config_run.subcommand,
        SubCommand::Run {
            file: "script.sgo".to_string()
        }
    );

    // 2. Check subcommand
    let args_check = vec!["sgo".to_string(), "check".to_string(), "test.sgo".to_string()];
    let config_check = CliConfig::parse_args(&args_check).expect("Parse check failed");
    assert_eq!(
        config_check.subcommand,
        SubCommand::Check {
            file: "test.sgo".to_string()
        }
    );

    // 3. Compile subcommand with explicit -o output
    let args_compile = vec![
        "sgo".to_string(),
        "compile".to_string(),
        "input.sgo".to_string(),
        "-o".to_string(),
        "out.sgb".to_string(),
    ];
    let config_compile = CliConfig::parse_args(&args_compile).expect("Parse compile failed");
    assert_eq!(
        config_compile.subcommand,
        SubCommand::Compile {
            file: "input.sgo".to_string(),
            output: Some("out.sgb".to_string())
        }
    );

    // 4. Watch subcommand with --once flag
    let args_watch = vec![
        "sgo".to_string(),
        "watch".to_string(),
        "live.sgo".to_string(),
        "--once".to_string(),
    ];
    let config_watch = CliConfig::parse_args(&args_watch).expect("Parse watch failed");
    assert_eq!(
        config_watch.subcommand,
        SubCommand::Watch {
            file: "live.sgo".to_string(),
            max_iterations: Some(1)
        }
    );

    // 5. Help subcommand
    let args_help = vec!["sgo".to_string(), "--help".to_string()];
    let config_help = CliConfig::parse_args(&args_help).expect("Parse help failed");
    assert_eq!(config_help.subcommand, SubCommand::Help);
}

#[test]
fn test_cli_run_sgo_script_execution() {
    let temp_dir = std::env::temp_dir();
    let script_path = temp_dir.join("test_run_script.sgo");

    let script_content = r#"
        let x: Int = 40;
        let y: Int = 2;
        let z: Int = x + y;
    "#;
    fs::write(&script_path, script_content).expect("Failed to write test sgo script");

    let result = run_cmd(script_path.to_str().unwrap()).expect("sgo run failed");
    assert!(matches!(result, VmResult::Halted(_)));

    let _ = fs::remove_file(script_path);
}

#[test]
fn test_cli_check_sgo_script_validation() {
    let temp_dir = std::env::temp_dir();

    // Valid script
    let valid_path = temp_dir.join("test_valid.sgo");
    fs::write(
        &valid_path,
        "let a: Int = 10; let b: Int = 20; let c: Int = a + b;",
    )
    .expect("Failed to write valid script");

    let check_result = check_cmd(valid_path.to_str().unwrap());
    assert!(check_result.is_ok(), "Valid script should pass check");

    // Invalid script
    let invalid_path = temp_dir.join("test_invalid.sgo");
    fs::write(&invalid_path, "let a: Int = ;;; invalid syntax")
        .expect("Failed to write invalid script");

    let invalid_check = check_cmd(invalid_path.to_str().unwrap());
    assert!(
        invalid_check.is_err(),
        "Invalid script should fail check"
    );

    let _ = fs::remove_file(valid_path);
    let _ = fs::remove_file(invalid_path);
}

#[test]
fn test_cli_compile_and_run_sgb_binary() {
    let temp_dir = std::env::temp_dir();
    let sgo_path = temp_dir.join("test_compile.sgo");
    let sgb_path = temp_dir.join("test_compile.sgb");

    let script_content = r#"
        let val: Int = 100;
        let res: Int = val * 2;
    "#;
    fs::write(&sgo_path, script_content).expect("Failed to write sgo script");

    let compiled_path = compile_cmd(
        sgo_path.to_str().unwrap(),
        Some(sgb_path.to_str().unwrap().to_string()),
    )
    .expect("Compile command failed");

    assert_eq!(compiled_path, sgb_path);
    assert!(sgb_path.exists(), "Compiled SGB file must exist");

    // Run the compiled binary .sgb file
    let run_result = run_cmd(sgb_path.to_str().unwrap()).expect("Running SGB binary failed");
    assert!(matches!(run_result, VmResult::Halted(_)));

    let _ = fs::remove_file(sgo_path);
    let _ = fs::remove_file(sgb_path);
}

#[test]
fn test_cli_watch_hot_reloading_state_preservation() {
    let temp_dir = std::env::temp_dir();
    let watch_path = temp_dir.join("test_watch_reload.sgo");

    // Initial script: writes 99 to persistent register R[16] via assembly instructions
    let initial_script = r#"
        LOADIMM 16 99
        LOADIMM 1 42
        HALT
    "#;
    fs::write(&watch_path, initial_script).expect("Failed to write initial watch script");

    // Run initial execution via watch_cmd with --once mode
    let vm = watch_cmd(watch_path.to_str().unwrap(), Some(1))
        .expect("Watch command initial run failed");

    assert_eq!(vm.registers[16], 99, "R[16] persistent register should be 99");
    assert_eq!(vm.registers[1], 42, "R[1] ephemeral register should be 42");

    // Update script file to simulate file modification
    let updated_script = r#"
        LOADIMM 17 88
        HALT
    "#;
    fs::write(&watch_path, updated_script).expect("Failed to update watch script");

    // Re-trigger watch_cmd / hot_reload test
    // To verify hot_reload behavior directly on vm:
    let new_bytecode = compile_source(updated_script).expect("Recompilation failed");
    let mut reloaded_vm = vm;
    reloaded_vm.hot_reload();

    // After hot_reload(): R[1] (ephemeral) is reset to 0, R[16] (persistent) is preserved
    assert_eq!(
        reloaded_vm.registers[1], 0,
        "Ephemeral register R[1] must be reset after hot_reload"
    );
    assert_eq!(
        reloaded_vm.registers[16], 99,
        "Persistent register R[16] must be preserved after hot_reload"
    );

    // Execute updated script on hot-reloaded VM
    let result = reloaded_vm.run(&new_bytecode).expect("Execution post reload failed");
    assert!(matches!(result, VmResult::Halted(_)));

    // R[16] remains 99, R[17] is now 88
    assert_eq!(reloaded_vm.registers[16], 99);
    assert_eq!(reloaded_vm.registers[17], 88);

    let _ = fs::remove_file(watch_path);
}

#[test]
fn test_cli_end_to_end_process_invocation() {
    let temp_dir = std::env::temp_dir();
    let sgo_file = temp_dir.join("test_e2e_cli.sgo");

    let script = r#"
        let count: Int = 5;
    "#;
    fs::write(&sgo_file, script).expect("Failed to write sgo file");

    let output = Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--",
            "run",
            sgo_file.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute cargo run command");

    assert!(
        output.status.success(),
        "CLI process execution failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("[RUN] Execution completed"),
        "Stdout should contain completion message: {}",
        stdout
    );

    let _ = fs::remove_file(sgo_file);
}
