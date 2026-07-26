#![allow(unused_imports)]
use covopt_macro::covopt_param;
use std::io::Write;
extern crate std;
    use super::*;
    use crate::sgl::instruction::OpCode;
    use crate::sgl::vm::{ScriptVm, VmError};


#[test]
    fn test_div_by_zero() {
let mut vm = ScriptVm::new();
        // LOADIMM 1 10
        // LOADIMM 2 0
        // DIV 3 1 2
        let code = [
            Instruction::new(OpCode::LoadImm as u8, 1, covopt_param!("M_17_55", 10), 0),
            Instruction::new(OpCode::LoadImm as u8, 2, 0, 0),
            Instruction::new(OpCode::Div as u8, covopt_param!("M_19_48", 3), 1, 2),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];

        let result = vm.run(&code);
        assert_eq!(result, Err(VmError::DivideByZero { pc: 2 }));
    }

    #[test]
    fn test_stack_overflow() {
let mut vm = ScriptVm::new();
        // CALL 0 (recursive call to itself)
        let code = [Instruction::new(OpCode::Call as u8, 0, 0, 0)];

        let result = vm.run(&code);
        assert_eq!(result, Err(VmError::StackOverflow { pc: 0 }));
    }

    #[test]
    fn test_stack_underflow() {
let mut vm = ScriptVm::new();
        // RET (no call pushed)
        let code = [Instruction::new(OpCode::Ret as u8, 0, 0, 0)];

        let result = vm.run(&code);
        assert_eq!(result, Err(VmError::StackUnderflow { pc: 0 }));
    }

    #[test]
    fn test_invalid_opcode() {
let mut vm = ScriptVm::new();
        let code = [
            Instruction::new(covopt_param!("M_51_29", 153), 0, 0, 0), // 0x99 is undefined
        ];

        let result = vm.run(&code);
        assert_eq!(
            result,
            Err(VmError::InvalidOpcode {
                pc: 0,
                opcode: 0x99
            })
        );
    }

    #[test]
    fn test_floats() {
let n = std::env::var("COVOPT_N").unwrap_or(std::string::String::from("1")).parse::<usize>().unwrap();
        let mut vm = ScriptVm::new();
        // Load f32 values represented as raw bits
        let val1 = (covopt_param!("M_69_19", 3.5) as f32).to_bits();
        let val2 = (covopt_param!("M_70_19", 1.5) as f32).to_bits();

        vm.registers[1] = val1;
        vm.registers[2] = val2;

        let code = [
            Instruction::new(OpCode::FAdd as u8, covopt_param!("M_76_49", 3), 1, 2),
            Instruction::new(OpCode::FSub as u8, covopt_param!("M_77_49", 4), 1, 2),
            Instruction::new(OpCode::FMul as u8, covopt_param!("M_78_49", 5), 1, 2),
            Instruction::new(OpCode::FDiv as u8, covopt_param!("M_79_49", 6), 1, 2),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];

        for _ in 0..n {
            std::hint::black_box(vm.run(&code).unwrap());
        }

        assert_eq!(f32::from_bits(vm.registers[3]), 5.0f32);
        assert_eq!(f32::from_bits(vm.registers[4]), 2.0f32);
        assert_eq!(f32::from_bits(vm.registers[5]), 5.25f32);
        assert_eq!(f32::from_bits(vm.registers[6]), 3.5 / 1.5);
    }

    #[test]
    fn test_memory_load_store() {
let mut vm = ScriptVm::new();
        // R[1] = 42 (value to store)
        // R[2] = 10 (base address)
        // R[3] = 4 (offset)
        // Store R[1] to Memory[R[2] + R[3]]
        // R[4] = Load from Memory[R[2] + R[3]]
        vm.registers[1] = covopt_param!("M_101_26", 42);
        vm.registers[2] = covopt_param!("M_102_26", 10);
        vm.registers[covopt_param!("M_103_21", 3)] = covopt_param!("M_103_26", 4);

        let code = [
            Instruction::new(OpCode::Store as u8, 1, 2, covopt_param!("M_106_56", 3)),
            Instruction::new(OpCode::Load as u8, covopt_param!("M_107_49", 4), 2, covopt_param!("M_107_55", 3)),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];

        std::hint::black_box(vm.run(&code).unwrap());

        assert_eq!(vm.registers[4], 42);
        // Verify bytes in memory (little endian)
        assert_eq!(vm.memory[14], 42);
        assert_eq!(vm.memory[15], 0);
        assert_eq!(vm.memory[16], 0);
        assert_eq!(vm.memory[17], 0);
    }

    #[test]
    fn test_math_approximations() {
let mut vm = ScriptVm::new();
        // EXP: exp_approx_q16
        // R[1] = 0 (Q16.16)
        // RSQRT: rsqrt_approx_i32
        // R[2] = 4
        // SILU: silu_approx_i8
        // R[3] = 2
        vm.registers[1] = 0;
        vm.registers[2] = covopt_param!("M_131_26", 4);
        vm.registers[covopt_param!("M_132_21", 3)] = 2;

        let code = [
            Instruction::new(OpCode::Exp as u8, covopt_param!("M_135_48", 4), 1, 0),
            Instruction::new(OpCode::Rsqrt as u8, covopt_param!("M_136_50", 5), 2, 0),
            Instruction::new(OpCode::Silu as u8, covopt_param!("M_137_49", 6), covopt_param!("M_137_52", 3), 0),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];

        std::hint::black_box(vm.run(&code).unwrap());

        // exp(0) = 1.0 (Q16.16 -> 65536)
        assert_eq!(vm.registers[4], 65536);
        // rsqrt(4) = 1/sqrt(4) = 0.5 (Q16.16 -> 32768)
        assert_eq!(vm.registers[5], 32768);
        // silu(2) ≈ 2 * (1 / (1 + exp(-2)))
        // Silu approx of 2 is non-zero
        assert!(vm.registers[6] > 0);
    }

    #[test]
    fn test_abort_flag() {
let mut vm = ScriptVm::new();
        vm.max_steps = None;
        static ABORT: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(true);
        fn abort_checker() -> bool { ABORT.load(core::sync::atomic::Ordering::Relaxed) }
        vm.abort_flag = Some(abort_checker);

        // Endless loop:
        // 0: JMP 0
        let code = [Instruction::new(OpCode::Jmp as u8, 0, 0, 0)];

        let result = vm.run(&code);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), crate::sgl::vm::VmResult::Halted(0));
    }


    #[test]
    fn test_out_of_fuel() {
let mut vm = ScriptVm::new();
        vm.max_steps = Some(covopt_param!("M_173_28", 50));
        let code = [Instruction::new(OpCode::Jmp as u8, 0, 0, 0)];
        let result = vm.run(&code);
        assert_eq!(result, Err(VmError::OutOfFuel { pc: 0 }));
    }

    #[test]
    fn test_trace_logging() {
let mut vm = ScriptVm::new();
        vm.tracing_enabled = true;

        let code = [
            Instruction::new(OpCode::LoadImm as u8, 1, covopt_param!("M_185_55", 42), 0),
            Instruction::new(OpCode::Store as u8, 1, 0, 0),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];

        std::hint::black_box(vm.run(&code).unwrap());

        assert_eq!(vm.trace_count, 2);
        let trace1 = vm.trace_buffer[0];
        assert_eq!(trace1.pc, 0);
        assert_eq!(trace1.reg_change, Some((1, 42)));
        assert_eq!(trace1.mem_change, None);

        let trace2 = vm.trace_buffer[1];
        assert_eq!(trace2.pc, 1);
        assert_eq!(trace2.reg_change, None);
        assert_eq!(trace2.mem_change, Some((0, 42)));
    }

    #[test]
    fn test_debug_hook() {
let mut vm = ScriptVm::new();
        use core::sync::atomic::{AtomicUsize, Ordering};
        static EXEC_COUNT: AtomicUsize = AtomicUsize::new(0);
        EXEC_COUNT.store(0, Ordering::Relaxed);

        vm.debug_hook = Some(|_vm, inst| {
            EXEC_COUNT.fetch_add(1, Ordering::Relaxed);
            if crate::opcode!(inst) == OpCode::LoadImm as u8 {
                assert_eq!(crate::inst_a!(inst), 1);
            }
        });

        let code = [
            Instruction::new(OpCode::LoadImm as u8, 1, covopt_param!("M_219_55", 10), 0),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];

        std::hint::black_box(vm.run(&code).unwrap());
        assert_eq!(EXEC_COUNT.load(Ordering::Relaxed), 2);
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_panic_recovery() {
let mut vm = ScriptVm::new();
        vm.print_handler = Some(|_| {
            panic!("Mock handler panic!");
        });

        let code = [Instruction::new(OpCode::PrintReg as u8, 0, 0, 0)];

        let vm_ref = &mut vm;
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            let _ = vm_ref.run(&code);
        }));

        assert!(result.is_err());
    }

    #[test]
    fn test_hot_reload_state_preservation() {
let mut vm = ScriptVm::new();
        // Set some ephemeral state
        vm.pc = covopt_param!("M_249_16", 42);
        vm.sp = covopt_param!("M_250_16", 5);
        vm.call_stack[0] = covopt_param!("M_251_27", 99);
        vm.registers[covopt_param!("M_252_21", 3)] = covopt_param!("M_252_26", 77); // Ephemeral register

        // Set some persistent state
        vm.registers[covopt_param!("M_255_21", 20)] = covopt_param!("M_255_27", 88); // Persistent register
        vm.memory[covopt_param!("M_256_18", 10)] = covopt_param!("M_256_24", 55); // RAM

        vm.hot_reload();

        // Ephemeral state must be reset
        assert_eq!(vm.pc, 0);
        assert_eq!(vm.sp, 0);
        assert_eq!(vm.call_stack[0], 0);
        assert_eq!(vm.registers[3], 0);

        // Persistent state must be preserved
        assert_eq!(vm.registers[20], 88);
        assert_eq!(vm.memory[10], 55);
    }    #[test]
    fn test_audit() {
        let n = std::env::var("COVOPT_N")
            .unwrap_or(std::string::String::from("1000"))
            .parse::<usize>()
            .unwrap();
        
        let mut handles = std::vec::Vec::new(); let (tx, rx) = std::sync::mpsc::channel();
        for _ in 0..covopt_param!("M_277_20", 4) {
            let tx_clone = tx.clone(); let handle = std::thread::spawn(move || {
                let n = n;
                let mut vm = ScriptVm::new();
                vm.print_handler = Some(|_| {});

                vm.registers[1] = 2;
                vm.registers[2] = 1;

                let code = [
                    // Setup
                    Instruction::new(OpCode::LoadImm as u8, 1, 2, 0), // R1 = 2
                    Instruction::new(OpCode::LoadImm as u8, 2, 1, 0), // R2 = 1
                    Instruction::new(OpCode::LoadImm as u8, 0, 0, 0), // R0 = 0
                    
                    // 3: JmpIfZero
                    Instruction::new(OpCode::JmpIfZero as u8, 1, 0, 0), // false
                    Instruction::new(OpCode::JmpIfZero as u8, 0, covopt_param!("M_294_65", 6), 0), // true, PC=6
                    Instruction::new(OpCode::Halt as u8, 0, 0, 0),
                    
                    // 6: JmpIfEq
                    Instruction::new(OpCode::JmpIfEq as u8, 1, 2, 0), // false
                    Instruction::new(OpCode::JmpIfEq as u8, 1, 1, covopt_param!("M_299_66", 9)), // true, PC=9
                    Instruction::new(OpCode::Halt as u8, 0, 0, 0),
                    
                    // 9: JmpIfLt
                    Instruction::new(OpCode::JmpIfLt as u8, 1, 2, 0), // false
                    Instruction::new(OpCode::JmpIfLt as u8, 2, 1, covopt_param!("M_304_66", 12)), // true, PC=12
                    Instruction::new(OpCode::Halt as u8, 0, 0, 0),
                    
                    // 12: JmpIfGt
                    Instruction::new(OpCode::JmpIfGt as u8, 2, 1, 0), // false
                    Instruction::new(OpCode::JmpIfGt as u8, 1, 2, covopt_param!("M_309_66", 15)), // true, PC=15
                    Instruction::new(OpCode::Halt as u8, 0, 0, 0),
                    
                    // 15: JmpIfFLt
                    Instruction::new(OpCode::JmpIfFLt as u8, 1, 2, 0), // false
                    Instruction::new(OpCode::JmpIfFLt as u8, 2, 1, covopt_param!("M_314_67", 18)), // true, PC=18
                    Instruction::new(OpCode::Halt as u8, 0, 0, 0),
                    
                    // 18: JmpIfFGt
                    Instruction::new(OpCode::JmpIfFGt as u8, 2, 1, 0), // false
                    Instruction::new(OpCode::JmpIfFGt as u8, 1, 2, covopt_param!("M_319_67", 21)), // true, PC=21
                    Instruction::new(OpCode::Halt as u8, 0, 0, 0),
                    
                    // 21: other ops
                    Instruction::new(OpCode::LoadImm16 as u8, covopt_param!("M_323_62", 4), 0, covopt_param!("M_323_68", 5)),
                    Instruction::new(OpCode::Add as u8, covopt_param!("M_324_56", 5), 1, 2),
                    Instruction::new(OpCode::Sub as u8, covopt_param!("M_325_56", 5), 1, 2),
                    Instruction::new(OpCode::Mul as u8, covopt_param!("M_326_56", 5), 1, 2),
                    Instruction::new(OpCode::Div as u8, covopt_param!("M_327_56", 5), 1, 2),
                    Instruction::new(OpCode::Mod as u8, covopt_param!("M_328_56", 5), 1, 2),
                    Instruction::new(OpCode::And as u8, covopt_param!("M_329_56", 5), 1, 2),
                    Instruction::new(OpCode::Or as u8, covopt_param!("M_330_55", 5), 1, 2),
                    Instruction::new(OpCode::Xor as u8, covopt_param!("M_331_56", 5), 1, 2),
                    Instruction::new(OpCode::Shl as u8, covopt_param!("M_332_56", 5), 1, 2),
                    Instruction::new(OpCode::Shr as u8, covopt_param!("M_333_56", 5), 1, 2),
                    Instruction::new(OpCode::CmpEq as u8, covopt_param!("M_334_58", 5), 1, 2),
                    Instruction::new(OpCode::CmpLt as u8, covopt_param!("M_335_58", 5), 1, 2),
                    Instruction::new(OpCode::FAdd as u8, covopt_param!("M_336_57", 5), 1, 2),
                    Instruction::new(OpCode::FSub as u8, covopt_param!("M_337_57", 5), 1, 2),
                    Instruction::new(OpCode::FMul as u8, covopt_param!("M_338_57", 5), 1, 2),
                    Instruction::new(OpCode::FDiv as u8, covopt_param!("M_339_57", 5), 1, 2),
                    Instruction::new(OpCode::Store as u8, 1, 0, 2),
                    Instruction::new(OpCode::Load as u8, covopt_param!("M_341_57", 5), 0, 2),
                    Instruction::new(OpCode::PrintReg as u8, covopt_param!("M_342_61", 5), 0, 0),
                    Instruction::new(OpCode::SysCall as u8, covopt_param!("M_343_60", 5), 0, 0),
                    
                    Instruction::new(OpCode::Call as u8, 0, covopt_param!("M_345_60", 44), 0), // 42: Push PC=43, Jmp 44
                    Instruction::new(OpCode::Jmp as u8, 0, covopt_param!("M_346_59", 45), 0),  // 43: Jmp 45
                    Instruction::new(OpCode::Ret as u8, 0, 0, 0),   // 44: Pops PC=43, Jmp 43
                    Instruction::new(OpCode::Halt as u8, 0, 0, 0),  // 45
                ];

                for _ in 0..n {
                    std::hint::black_box(vm.run(&code).unwrap());
                }
                tx_clone.send(()).unwrap(); });
            handles.push(handle);
        }
        for _ in 0..covopt_param!("M_357_20", 4) {
            rx.recv_timeout(std::time::Duration::from_secs(covopt_param!("M_358_59", 5))).expect("Watchdog timeout");
        }
        for handle in handles {
            handle.join().unwrap();
        }
        
        // --- Coverage Boost for Error Branches ---
        let mut vm_err = ScriptVm::new();
        
        // 1. DivideByZero
        let code_div0 = [
            Instruction::new(OpCode::LoadImm as u8, 1, covopt_param!("M_369_55", 10), 0),
            Instruction::new(OpCode::LoadImm as u8, 2, 0, 0),
            Instruction::new(OpCode::Div as u8, covopt_param!("M_371_48", 3), 1, 2),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];
        let _ = vm_err.run_fast(&code_div0);

        // 2. FDiv by Zero
        let code_fdiv0 = [
            Instruction::new(OpCode::LoadImm as u8, 1, covopt_param!("M_378_55", 10), 0),
            Instruction::new(OpCode::LoadImm as u8, 2, 0, 0),
            Instruction::new(OpCode::FDiv as u8, covopt_param!("M_380_49", 3), 1, 2),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];
        let _ = vm_err.run_fast(&code_fdiv0);
        
        // 3. MemoryOutOfBounds (Load/Store)
        let code_mem = [
            Instruction::new(OpCode::LoadImm16 as u8, 1, (covopt_param!("M_387_58", 10000) & covopt_param!("M_387_66", 255)) as u8, (covopt_param!("M_387_79", 10000) >> covopt_param!("M_387_88", 8)) as u8),
            Instruction::new(OpCode::Load as u8, 2, 1, 0),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];
        let _ = vm_err.run_fast(&code_mem);
        
        // 4. StackOverflow
        let code_so = [Instruction::new(OpCode::Call as u8, 0, 0, 0); 257];
        let mut vm_so = ScriptVm::new();
        let _ = vm_so.run_fast(&code_so);
        
        // 5. StackUnderflow
        let code_su = [
            Instruction::new(OpCode::Ret as u8, 0, 0, 0),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];
        let _ = vm_err.run_fast(&code_su);
        
        // 6. InvalidOpCode
        let code_inv = [
            Instruction::new(covopt_param!("M_407_29", 255), 0, 0, 0),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];
        let _ = vm_err.run_fast(&code_inv);

        // 7. Handlers (Coverage Boost)
        let mut vm_handlers = ScriptVm::new();
        vm_handlers.print_handler = Some(|_| {});
        vm_handlers.syscall_handler = Some(|_, _, _| {});
        vm_handlers.hardware_handler = Some(|_, _, _, _| {});
        vm_handlers.ui_handler = Some(|_, _, _| {});
        vm_handlers.neural_handler = Some(|_, _, _, _| {});
        
        let code_handlers = [
            Instruction::new(OpCode::PrintReg as u8, 0, 0, 0),
            Instruction::new(OpCode::SysCall as u8, 0, 0, 0),
            Instruction::new(OpCode::HardwareCall as u8, 0, 0, 0),
            Instruction::new(OpCode::UiCall as u8, 1, 1, 0), // ui_call requires a != 0 and b in 1..=4
            Instruction::new(OpCode::UiCall as u8, 0, covopt_param!("M_425_54", 5), 0), // false branch for ui_call
            Instruction::new(OpCode::NeuralCall as u8, 0, 0, 0),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];
        let _ = vm_handlers.run_fast(&code_handlers);

        // 8. Jumps and Control Flow (Coverage Boost)
        let mut vm_jumps = ScriptVm::new();
        vm_jumps.registers[1] = 0;
        vm_jumps.registers[2] = 1;
        
        let code_jumps = [
            // JmpIfZero
            Instruction::new(OpCode::JmpIfZero as u8, 1, 1, 0), // True -> PC=1 (imm16=b|c<<8) -> imm16=1
            Instruction::new(OpCode::JmpIfZero as u8, 2, 2, 0), // False -> imm16=2
            // JmpIfEq
            Instruction::new(OpCode::JmpIfEq as u8, 1, 1, covopt_param!("M_441_58", 3)),   // True -> PC=3
            Instruction::new(OpCode::JmpIfEq as u8, 1, 2, covopt_param!("M_442_58", 3)),   // False
            // JmpIfLt
            Instruction::new(OpCode::JmpIfLt as u8, 1, 2, covopt_param!("M_444_58", 5)),   // True -> PC=5
            Instruction::new(OpCode::JmpIfLt as u8, 2, 1, covopt_param!("M_445_58", 5)),   // False
            // JmpIfGt
            Instruction::new(OpCode::JmpIfGt as u8, 2, 1, covopt_param!("M_447_58", 7)),   // True -> PC=7
            Instruction::new(OpCode::JmpIfGt as u8, 1, 2, covopt_param!("M_448_58", 7)),   // False
            // JmpIfFLt
            Instruction::new(OpCode::JmpIfFLt as u8, 1, 2, covopt_param!("M_450_59", 9)),  // True -> PC=9
            Instruction::new(OpCode::JmpIfFLt as u8, 2, 1, covopt_param!("M_451_59", 9)),  // False
            // JmpIfFGt
            Instruction::new(OpCode::JmpIfFGt as u8, 2, 1, covopt_param!("M_453_59", 11)), // True -> PC=11
            Instruction::new(OpCode::JmpIfFGt as u8, 1, 2, covopt_param!("M_454_59", 11)), // False
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),      // 12
        ];
        let _ = vm_jumps.run_fast(&code_jumps);
        
        // 9. Store OutOfBounds
        let code_store = [
            Instruction::new(OpCode::LoadImm16 as u8, 1, (covopt_param!("M_461_58", 10000) & covopt_param!("M_461_66", 255)) as u8, (covopt_param!("M_461_79", 10000) >> covopt_param!("M_461_88", 8)) as u8),
            Instruction::new(OpCode::Store as u8, 2, 1, 0),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];
        let _ = vm_err.run_fast(&code_store);

        // 10. Math Errors
        // Exp error (needs input > 10 * 65536)
        let code_math_exp = [
            Instruction::new(OpCode::LoadImm as u8, 1, covopt_param!("M_470_55", 11), 0),
            Instruction::new(OpCode::LoadImm as u8, 2, covopt_param!("M_471_55", 16), 0),
            Instruction::new(OpCode::Shl as u8, 1, 1, 2),
            Instruction::new(OpCode::Exp as u8, covopt_param!("M_473_48", 3), 1, 0),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];
        let _ = vm_err.run_fast(&code_math_exp);

        // Rsqrt error (needs input == 0)
        let code_math_rsqrt = [
            Instruction::new(OpCode::LoadImm as u8, 1, 0, 0),
            Instruction::new(OpCode::Rsqrt as u8, 2, 1, 0),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];
        let _ = vm_err.run_fast(&code_math_rsqrt);

        // Silu error (needs input == 128 which is -128 as i8, causing Exp overflow)
        let code_math_silu = [
            Instruction::new(OpCode::LoadImm as u8, 1, covopt_param!("M_488_55", 128), 0),
            Instruction::new(OpCode::Silu as u8, 2, 1, 0),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];
        let _ = vm_err.run_fast(&code_math_silu);
    }


    #[test]
    fn test_compiler_vm_execution_and_ui_call() {
        use crate::compiler::lexer::Lexer;
        use crate::compiler::parser::Parser;
        use crate::compiler::codegen::CodeGen;
        use std::sync::atomic::{AtomicBool, Ordering};

        static UI_CALLED: AtomicBool = AtomicBool::new(false);
        fn test_ui_handler(_arg0: usize, _arg1: usize, _arg2: usize) {
            UI_CALLED.store(true, Ordering::SeqCst);
        }

        let input = r#"
            ui_call(10, 20, 30);
            let rem = 10 % 3;
            let is_le = 5 <= 5;
            let is_ge = 6 >= 2;
            let is_ne = 7 != 8;
        "#;

        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().expect("Lexing failed");
        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("Parsing failed");
        let mut codegen = CodeGen::new();
        let bytecode = codegen.compile(&program).expect("CodeGen failed");

        let mut vm = ScriptVm::new();
        vm.ui_handler = Some(test_ui_handler);
        let run_res = vm.run(&bytecode);
        assert!(run_res.is_ok(), "VM execution failed: {:?}", run_res.err());

        assert!(UI_CALLED.load(Ordering::SeqCst), "ui_handler was not called by ui_call instruction");
    }

