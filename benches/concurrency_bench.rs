use arc_swap::ArcSwap;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use script_go::assembler::parse_asm;
use script_go::instruction::Instruction;
use script_go::vm::ScriptVm;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::thread;

/// A mock HFT Gateway demonstrating RCU hot-reloading
struct Gateway {
    script: ArcSwap<Vec<Instruction>>,
}

impl Gateway {
    pub fn new(source: &str) -> Self {
        let code = parse_asm(source).unwrap();
        Self {
            script: ArcSwap::from_pointee(code),
        }
    }

    #[inline(always)]
    pub fn execute(&self) -> u32 {
        // RCU fast path: Load the Arc pointer without locking
        let code_guard = self.script.load();
        let mut vm = ScriptVm::new();
        vm.run(&code_guard).unwrap()
    }

    pub fn hot_reload(&self, source: &str) {
        let new_code = parse_asm(source).unwrap();
        self.script.store(Arc::new(new_code));
    }
}

fn bench_hot_reload_contention(c: &mut Criterion) {
    let mut group = c.benchmark_group("hft_hot_reload");
    let gateway = Arc::new(Gateway::new("LOADIMM 1 50\nADD 2 1 1\nHALT"));

    // Scenario 1: Uncontended Execution Latency
    group.bench_function("uncontended_e2e", |b| {
        b.iter(|| {
            black_box(gateway.execute());
        });
    });

    // Scenario 2: High Contention with Hot-Reloads
    let running = Arc::new(AtomicBool::new(true));
    
    // Spawn a writer thread that constantly hot-reloads the script 
    let writer_running = running.clone();
    let gw_writer = gateway.clone();
    let writer_handle = thread::spawn(move || {
        let mut toggle = false;
        while writer_running.load(Ordering::Relaxed) {
            let script = if toggle {
                "LOADIMM 1 100\nSUB 2 1 1\nHALT"
            } else {
                "LOADIMM 1 50\nADD 2 1 1\nHALT"
            };
            gw_writer.hot_reload(script);
            toggle = !toggle;
        }
    });

    // Spawn 8 background reader threads
    let mut reader_handles = vec![];
    for _ in 0..8 {
        let reader_running = running.clone();
        let gw_reader = gateway.clone();
        reader_handles.push(thread::spawn(move || {
            while reader_running.load(Ordering::Relaxed) {
                black_box(gw_reader.execute());
            }
        }));
    }

    group.bench_function("contended_hot_reload", |b| {
        b.iter(|| {
            black_box(gateway.execute());
        });
    });

    // Clean up background threads
    running.store(false, Ordering::Relaxed);
    writer_handle.join().unwrap();
    for h in reader_handles {
        h.join().unwrap();
    }

    group.finish();
}

criterion_group!(benches, bench_hot_reload_contention);
criterion_main!(benches);
