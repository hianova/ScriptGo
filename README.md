# 🚀 ScriptGo

**ScriptGo** is an aerospace-grade, hyper-optimized, Register-based Virtual Machine designed for absolute zero-allocation execution and High-Frequency Trading (HFT) latency standards.

## Features

- **Turing-Complete ISA**: 32-bit fixed-length RISC Instruction Set.
- **Zero-Allocation**: Strictly `#![no_std]` compatible. Entire VM state resides in pre-allocated static arrays (256 Registers, 64 Call Stack Depth).
- **Extreme Latency**: Instruction dispatch and execution latency sits around ~1.19ns per instruction (near 1 GIPS in pure software).
- **HFT Gateway**: Built-in lock-free RCU (Read-Copy-Update) hot-reload mechanism via `arc-swap`. Uncontended parse+execution latency is ~27ns; Contended Hot-Reload latency maxes at ~63ns with absolutely zero microsecond-level spikes.
- **In-Process Embedded**: Say goodbye to CGI `fork/exec` overhead. ScriptGo is designed to be fully embedded inside Tokio/Axum worker threads.

## Benchmarks

| Scenario | Latency | Throughput / Notes |
| :--- | :--- | :--- |
| Single Instruction | ~1.19 ns | ~1 GIPS Theoretical Limit |
| Uncontended E2E | ~27.6 ns | Parse & Execution of script via RCU |
| Hot-Reload Contention | ~63.8 ns | 100 threads reading + 1 thread hot-swapping |

*(Benchmarked on local machine using `criterion` and `covopt` aerospace-grade audit)*

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

let code = parse_asm(script);
let mut vm = ScriptVm::new();
vm.run(&code);

assert_eq!(vm.registers[3], 15);
```

## Architecture

- **`instruction.rs`**: Defines the 32-bit `[OpCode, RegA, RegB, RegC]` RISC format.
- **`vm.rs`**: The pure `no_std` Register VM execution loop.
- **`assembler.rs`**: Dynamic string parsing to `Vec<Instruction>`.
- **`gateway.rs` (Examples)**: In-Process RCU Gateway showcasing zero-downtime hot-reloads.
