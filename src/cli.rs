use crate::assembler::parse_asm;
use crate::binary::{deserialize_sgb, serialize_sgb, SGB_MAGIC};
use crate::compiler::codegen::CodeGen;
use crate::compiler::lexer::Lexer;
use crate::compiler::parser::Parser;
use crate::sgl::host_handlers::HostContext;
use crate::sgl::instruction::Instruction;
use crate::sgl::ui_engine::UiDispatcher;
use crate::sgl::vm::{ScriptVm, TraceStep, VmResult};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, SystemTime};
use std::{eprintln, println};


/// Supported CLI Subcommands for ScriptGo
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubCommand {
    Run { file: String },
    Check { file: String },
    Compile { file: String, output: Option<String> },
    Watch { file: String, max_iterations: Option<usize> },
    Replay { file: String },
    Build { input: String, output: String },
    Help,
}

/// CLI Argument Parser Configuration
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliConfig {
    pub subcommand: SubCommand,
}

impl CliConfig {
    pub fn parse_args(args: &[String]) -> Result<Self, String> {
        if args.len() <= 1 {
            return Ok(CliConfig {
                subcommand: SubCommand::Help,
            });
        }

        let subcommand_str = args[1].as_str();

        match subcommand_str {
            "run" => {
                if args.len() < 3 {
                    return Err("Usage: sgo run <file.sgo>".to_string());
                }
                Ok(CliConfig {
                    subcommand: SubCommand::Run {
                        file: args[2].clone(),
                    },
                })
            }
            "check" => {
                if args.len() < 3 {
                    return Err("Usage: sgo check <file.sgo>".to_string());
                }
                Ok(CliConfig {
                    subcommand: SubCommand::Check {
                        file: args[2].clone(),
                    },
                })
            }
            "compile" => {
                if args.len() < 3 {
                    return Err("Usage: sgo compile <file.sgo> [-o output.sgb]".to_string());
                }
                let mut file = None;
                let mut output = None;
                let mut idx = 2;
                while idx < args.len() {
                    if args[idx] == "-o" || args[idx] == "--output" {
                        if idx + 1 < args.len() {
                            output = Some(args[idx + 1].clone());
                            idx += 2;
                        } else {
                            return Err("Missing argument for -o / --output".to_string());
                        }
                    } else if file.is_none() {
                        file = Some(args[idx].clone());
                        idx += 1;
                    } else {
                        idx += 1;
                    }
                }
                let file = file.ok_or_else(|| "Usage: sgo compile <file.sgo> [-o output.sgb]".to_string())?;
                Ok(CliConfig {
                    subcommand: SubCommand::Compile { file, output },
                })
            }
            "watch" => {
                if args.len() < 3 {
                    return Err("Usage: sgo watch <file.sgo> [--once]".to_string());
                }
                let file = args[2].clone();
                let mut max_iterations = None;
                if args.iter().any(|arg| arg == "--once") {
                    max_iterations = Some(1);
                }
                Ok(CliConfig {
                    subcommand: SubCommand::Watch {
                        file,
                        max_iterations,
                    },
                })
            }
            "--replay" => {
                if args.len() < 3 {
                    return Err("Usage: sgo --replay <trace.json>".to_string());
                }
                Ok(CliConfig {
                    subcommand: SubCommand::Replay {
                        file: args[2].clone(),
                    },
                })
            }
            "--build" => {
                if args.len() < 4 {
                    return Err("Usage: sgo --build <input> <output>".to_string());
                }
                Ok(CliConfig {
                    subcommand: SubCommand::Build {
                        input: args[2].clone(),
                        output: args[3].clone(),
                    },
                })
            }
            "--help" | "-h" | "help" => Ok(CliConfig {
                subcommand: SubCommand::Help,
            }),
            _ => {
                if Path::new(subcommand_str).exists()
                    || subcommand_str.ends_with(".sgo")
                    || subcommand_str.ends_with(".sgb")
                {
                    Ok(CliConfig {
                        subcommand: SubCommand::Run {
                            file: subcommand_str.to_string(),
                        },
                    })
                } else {
                    Err(format!("Unknown subcommand or file not found: {}", subcommand_str))
                }
            }
        }
    }
}

/// Compiles source text (high-level SGL script or assembly) into VM instructions.
pub fn compile_source(source: &str) -> Result<Vec<Instruction>, String> {
    let mut lexer = Lexer::new(source);
    match lexer.tokenize() {
        Ok(tokens) => {
            let mut parser = Parser::new(tokens);
            match parser.parse() {
                Ok(program) => {
                    let mut codegen = CodeGen::new();
                    match codegen.compile(&program) {
                        Ok(bytecode) => Ok(bytecode),
                        Err(e) => {
                            if let Ok(asm_code) = parse_asm(source) {
                                Ok(asm_code)
                            } else {
                                Err(format!("CodeGen compilation error: {}", e))
                            }
                        }
                    }
                }
                Err(e) => {
                    if let Ok(asm_code) = parse_asm(source) {
                        Ok(asm_code)
                    } else {
                        Err(format!("Parser error: {}", e))
                    }
                }
            }
        }
        Err(e) => {
            if let Ok(asm_code) = parse_asm(source) {
                Ok(asm_code)
            } else {
                Err(format!("Lexer error: {}", e))
            }
        }
    }
}

/// Executes `sgo run <file>` command
pub fn run_cmd(file_path: &str) -> Result<VmResult, String> {
    let bytes = fs::read(file_path)
        .map_err(|e| format!("Failed to read file '{}': {}", file_path, e))?;

    let bytecode = if bytes.len() >= 4 && bytes[0..4] == SGB_MAGIC {
        let (code, _, _) = deserialize_sgb(&bytes)
            .map_err(|e| format!("Failed to deserialize SGB binary: {}", e))?;
        code
    } else {
        let source = String::from_utf8(bytes)
            .map_err(|e| format!("File content is not valid UTF-8: {}", e))?;
        compile_source(&source)?
    };

    let mut vm = ScriptVm::new();
    vm.register_host_context(HostContext::new());
    vm.register_ui_dispatcher(UiDispatcher::new());

    let result = vm
        .run(&bytecode)
        .map_err(|e| format!("VM Execution Error: {:?}", e))?;

    println!("[RUN] Execution completed: {:?}", result);
    Ok(result)
}

/// Executes `sgo check <file>` command
pub fn check_cmd(file_path: &str) -> Result<(), String> {
    let bytes = fs::read(file_path)
        .map_err(|e| format!("Failed to read file '{}': {}", file_path, e))?;

    if bytes.len() >= 4 && bytes[0..4] == SGB_MAGIC {
        deserialize_sgb(&bytes)
            .map_err(|e| format!("SGB binary check failed: {}", e))?;
    } else {
        let source = String::from_utf8(bytes)
            .map_err(|e| format!("File content is not valid UTF-8: {}", e))?;
        compile_source(&source)?;
    }

    println!("[CHECK] OK: {} (Syntax and types valid)", file_path);
    Ok(())
}

/// Executes `sgo compile <file> [-o output]` command
pub fn compile_cmd(file_path: &str, output_path: Option<String>) -> Result<PathBuf, String> {
    let source = fs::read_to_string(file_path)
        .map_err(|e| format!("Failed to read source file '{}': {}", file_path, e))?;

    let bytecode = compile_source(&source)?;
    let binary_bytes = serialize_sgb(&bytecode, 10000, &[]);

    let target_path = match output_path {
        Some(out) => PathBuf::from(out),
        None => {
            let path = Path::new(file_path);
            if path.extension().and_then(|s| s.to_str()) == Some("sgo") {
                path.with_extension("sgb")
            } else {
                let mut p = path.to_path_buf();
                p.set_extension("sgb");
                p
            }
        }
    };

    fs::write(&target_path, &binary_bytes)
        .map_err(|e| format!("Failed to write SGB file '{}': {}", target_path.display(), e))?;

    println!(
        "[COMPILE] Compiled {} -> {} ({} bytes, {} instructions)",
        file_path,
        target_path.display(),
        binary_bytes.len(),
        bytecode.len()
    );

    Ok(target_path)
}

/// Executes `sgo watch <file>` command with hot-reloading capability
pub fn watch_cmd(file_path: &str, max_iterations: Option<usize>) -> Result<ScriptVm, String> {
    let mut vm = ScriptVm::new();
    vm.register_host_context(HostContext::new());
    vm.register_ui_dispatcher(UiDispatcher::new());

    let path = Path::new(file_path);
    if !path.exists() {
        return Err(format!("Watch target file '{}' does not exist", file_path));
    }

    let read_and_compile = || -> Result<Vec<Instruction>, String> {
        let source = fs::read_to_string(file_path)
            .map_err(|e| format!("Failed to read file '{}': {}", file_path, e))?;
        compile_source(&source)
    };

    let initial_bytecode = read_and_compile()?;
    let result = vm
        .run(&initial_bytecode)
        .map_err(|e| format!("Initial VM execution error: {:?}", e))?;
    println!("[WATCH] Initial execution finished for {}: {:?}", file_path, result);

    let mut last_mtime = fs::metadata(file_path)
        .and_then(|m| m.modified())
        .unwrap_or_else(|_| SystemTime::now());

    let mut iterations = 0;
    loop {
        if let Some(limit) = max_iterations {
            iterations += 1;
            if iterations >= limit {
                break;
            }
        }

        thread::sleep(Duration::from_millis(50));

        let current_mtime = fs::metadata(file_path)
            .and_then(|m| m.modified())
            .ok();

        if let Some(mtime) = current_mtime
            && mtime > last_mtime
        {
                last_mtime = mtime;
                println!("[WATCH] File modification detected in {}. Recompiling...", file_path);

                match read_and_compile() {
                    Ok(bytecode) => {
                        vm.hot_reload();
                        match vm.run(&bytecode) {
                            Ok(res) => {
                                println!(
                                    "[WATCH] Hot-reload & execution successful for {}: {:?}",
                                    file_path, res
                                );
                            }
                            Err(e) => {
                                eprintln!("[WATCH] VM execution error post hot-reload: {:?}", e);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("[WATCH] Recompilation error: {}", e);
                    }
                }
        }
    }

    Ok(vm)
}

/// Legacy command: Replay VM execution trace
pub fn replay_trace(path: &str) -> Result<(), String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read trace file '{}': {}", path, e))?;
    let trace: Vec<TraceStep> = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse JSON trace from '{}': {}", path, e))?;

    println!("⏱️  Replaying trace from: {}", path);
    println!("--------------------------------------------------");
    for (i, step) in trace.iter().enumerate() {
        let change_str = if let Some((reg, val)) = step.reg_change {
            format!("R[{}] -> {}", reg, val)
        } else if let Some((addr, val)) = step.mem_change {
            format!("RAM[{}] -> {}", addr, val)
        } else {
            "No State Mutation".to_string()
        };

        println!(
            "[#{}] PC: {:03} | INST: 0x{:08X} | {}",
            i, step.pc, step.inst, change_str
        );
    }
    println!("--------------------------------------------------");
    println!("✅ Trace replay completed successfully!");
    Ok(())
}

/// Legacy command: Assemble source file into binary
pub fn build_asm(input_path: &str, output_path: &str) -> Result<(), String> {
    let content = fs::read_to_string(input_path)
        .map_err(|e| format!("Failed to read input file '{}': {}", input_path, e))?;
    let code = parse_asm(&content)
        .map_err(|e| format!("Assembly parse error: {:?}", e))?;
    let mut bytes = Vec::new();
    for inst in code {
        bytes.extend_from_slice(&inst.0.to_le_bytes());
    }
    fs::write(output_path, &bytes)
        .map_err(|e| format!("Failed to write output file '{}': {}", output_path, e))?;
    println!("Successfully built {} -> {}", input_path, output_path);
    Ok(())
}

/// Prints CLI usage help text
pub fn print_help() {
    println!("ScriptGo (SGL) DX CLI Tool");
    println!("Usage:");
    println!("  sgo run <file.sgo>                Compile and execute SGL script on ScriptVm");
    println!("  sgo check <file.sgo>              Parse and type-check SGL script without running");
    println!("  sgo compile <file.sgo> [-o out]   Compile SGL script into binary bytecode (.sgb)");
    println!("  sgo watch <file.sgo> [--once]     Watch SGL script, recompile & hot-reload on change");
    println!("  sgo --replay <trace.json>         Replay VM execution trace");
    println!("  sgo --build <in.asm> <out.sgb>    Assemble SGL assembly source into bytecode");
}

/// Main entry point for CLI command dispatching
pub fn run_cli(args: Vec<String>) -> Result<(), String> {
    let config = CliConfig::parse_args(&args)?;
    match config.subcommand {
        SubCommand::Run { file } => {
            run_cmd(&file)?;
        }
        SubCommand::Check { file } => {
            check_cmd(&file)?;
        }
        SubCommand::Compile { file, output } => {
            compile_cmd(&file, output)?;
        }
        SubCommand::Watch { file, max_iterations } => {
            watch_cmd(&file, max_iterations)?;
        }
        SubCommand::Replay { file } => {
            replay_trace(&file)?;
        }
        SubCommand::Build { input, output } => {
            build_asm(&input, &output)?;
        }
        SubCommand::Help => {
            print_help();
        }
    }
    Ok(())
}
