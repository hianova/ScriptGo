# 🚀 ScriptGo

**ScriptGo** is an aerospace-grade, hyper-optimized, Register-based Virtual Machine designed for absolute zero-allocation execution and High-Frequency Trading (HFT) latency standards.

## Features

- **Turing-Complete ISA**: 32-bit fixed-length RISC Instruction Set with boundary checking (`Result` based execution).
- **Zero-Allocation**: Strictly `#![no_std]` compatible. Entire VM state resides in pre-allocated static arrays (256 Registers, 64 Call Stack Depth).
- **Extreme Latency**: Instruction dispatch and execution latency sits around ~3.8ns per instruction (even with full memory boundary and arithmetic panic protections).
- **HFT Gateway**: Built-in lock-free RCU (Read-Copy-Update) hot-reload mechanism via `arc-swap`. Uncontended parse+execution latency is ~28ns; Contended Hot-Reload latency maxes at ~67ns with absolutely zero microsecond-level spikes.
- **Tauri Native UI Engine**: Complete replacement for Flutter & React Native. ScriptGo drives Tauri WebView purely via IPC with `UiCall (0xFE)` zero-copy events, rendering highly optimized Virtual DOM without heavy JS frameworks.
- **Zero-Downtime OTA (Over-The-Air)**: Business and UI logic encapsulated in `.sgo` scripts can be reloaded in under 250ns, achieving true zero-downtime hot reloading for desktop and mobile apps without App Store updates.

## Benchmarks

| Scenario | Latency | Throughput / Notes |
| :--- | :--- | :--- |
| Single Instruction | ~3.8 ns | Robust safety checks included |
| Uncontended E2E | ~28.6 ns | Parse & Execution of script via RCU |
| Hot-Reload Contention | ~65.1 ns | 100 threads reading + 1 thread hot-swapping |

*(Benchmarked on local machine using `criterion` and `covopt` aerospace-grade audit. CovOpt verified O(1) space/time complexity with Entropy Score of 15.0/100.0)*

## Usage

```rust
use script_go::assembler::parse_asm;
use script_go::vm::ScriptVm;

let script = r#"
    LOADIMM 1 5
    LOADIMM 2 10
    ADD 3 1 2
    HALT
"#;

let code = parse_asm(script).unwrap();
let mut vm = ScriptVm::new();
vm.run(&code).unwrap();

assert_eq!(vm.registers[3], 15);
```

## Architecture

- **`instruction.rs`**: Defines the 32-bit `[OpCode, RegA, RegB, RegC]` RISC format (Includes `UiCall`).
- **`vm.rs`**: The pure `no_std` Register VM execution loop with strict error boundary protections.
- **`assembler.rs`**: Dynamic string parsing to `Result<Vec<Instruction>, AsmError>`.
- **`examples/tauri_framework`**: The ultimate native UI engine combining ScriptGo and Tauri IPC.
- **`examples/markdown_notes`**: The hell-level validation product demonstrating 10,000 items virtual scrolling and zero-downtime OTA logic swaps.
