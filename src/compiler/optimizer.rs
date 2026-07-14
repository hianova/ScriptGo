use crate::compiler::ir::*;
use no_std_tool::collections::HashMap;

pub struct Optimizer;

impl Optimizer {
    pub fn optimize(func: &mut FunctionIR) {
        Self::constant_folding(func);
        Self::dead_code_elimination(func);
    }

    fn constant_folding(func: &mut FunctionIR) {
        let mut constants = HashMap::new();
        
        for block in &mut func.blocks {
            let mut i = 0;
            while i < block.insts.len() {
                // Using Rust 1.95+ if let guard
                match block.insts[i].op.clone() {
                    Op::LoadImm(v) => {
                        constants.insert(block.insts[i].id, v);
                    }
                    Op::Add(v1, v2) if constants.contains_key(&v1) && constants.contains_key(&v2) => {
                        let val = constants[&v1] + constants[&v2];
                        constants.insert(block.insts[i].id, val);
                        block.insts[i].op = Op::LoadImm(val);
                    }
                    Op::Sub(v1, v2) if constants.contains_key(&v1) && constants.contains_key(&v2) => {
                        let val = constants[&v1] - constants[&v2];
                        constants.insert(block.insts[i].id, val);
                        block.insts[i].op = Op::LoadImm(val);
                    }
                    Op::Mul(v1, v2) if constants.contains_key(&v1) && constants.contains_key(&v2) => {
                        let val = constants[&v1] * constants[&v2];
                        constants.insert(block.insts[i].id, val);
                        block.insts[i].op = Op::LoadImm(val);
                    }
                    Op::Div(v1, v2) if constants.contains_key(&v1) && constants.contains_key(&v2) => {
                        let divisor = constants[&v2];
                        if divisor != 0 {
                            let val = constants[&v1] / divisor;
                            constants.insert(block.insts[i].id, val);
                            block.insts[i].op = Op::LoadImm(val);
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
        }
    }

    fn dead_code_elimination(func: &mut FunctionIR) {
        // Very simple DCE: if a value is not used, remove it (except FFI/Return which have side effects)
        let mut used = HashMap::new();
        
        for block in &func.blocks {
            for inst in &block.insts {
                match &inst.op {
                    Op::Add(v1, v2) | Op::Sub(v1, v2) | Op::Mul(v1, v2) | Op::Div(v1, v2) | Op::TensorMul(v1, v2) => {
                        *used.entry(*v1).or_insert(0) += 1;
                        *used.entry(*v2).or_insert(0) += 1;
                    }
                    Op::FfiCall { args, .. } => {
                        for arg in args {
                            *used.entry(*arg).or_insert(0) += 1;
                        }
                    }
                    Op::Return(Some(v)) => {
                        *used.entry(*v).or_insert(0) += 1;
                    }
                    Op::DbFilter(v1, _) => {
                        *used.entry(*v1).or_insert(0) += 1;
                    }
                    _ => {}
                }
            }
        }

        for block in &mut func.blocks {
            block.insts.retain(|inst| {
                match inst.op {
                    // Side effect ops are kept
                    Op::FfiCall { .. } | Op::Return(_) | Op::DbFilter(..) | Op::TensorMul(..) => true,
                    // Keep if used
                    _ => used.contains_key(&inst.id) && used[&inst.id] > 0,
                }
            });
        }
    }
}
