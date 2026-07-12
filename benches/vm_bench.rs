use criterion::{black_box, criterion_group, criterion_main, Criterion};
use script_go::instruction::{Instruction, OpCode};
use script_go::vm::ScriptVm;

fn bench_vm_execution(c: &mut Criterion) {
    let mut group = c.benchmark_group("vm_execution");
    
    let code = vec![
        Instruction::new(OpCode::LoadImm as u8, 1, 250, 0), // 0: R[1] = 250
        Instruction::new(OpCode::LoadImm as u8, 2, 1, 0),   // 1: R[2] = 1
        Instruction::new(OpCode::Sub as u8, 1, 1, 2),       // 2: R[1] = R[1] - R[2]
        Instruction::new(OpCode::JmpIfZero as u8, 1, 5, 0), // 3: If R[1]==0, goto 5
        Instruction::new(OpCode::Jmp as u8, 0, 2, 0),       // 4: Goto 2
        Instruction::new(OpCode::Halt as u8, 0, 0, 0),      // 5: Halt
    ];

    group.bench_function("simple_loop_250_iters", |b| {
        let mut vm = ScriptVm::new();
        b.iter(|| {
            vm.run(black_box(&code)).unwrap();
        });
    });

    group.finish();
}

criterion_group!(benches, bench_vm_execution);
criterion_main!(benches);
